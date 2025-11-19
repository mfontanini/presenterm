use crate::{
    code::{
        execute::{ExecutionHandle, ExecutionState, LanguageSnippetExecutor, ProcessStatus},
        snippet::Snippet,
    },
    markdown::{
        elements::{Line, Text},
        text_style::{Colors, TextStyle},
    },
    render::{
        operation::{
            AsRenderOperations, BlockLine, Pollable, PollableState, RenderAsync, RenderAsyncStartPolicy,
            RenderOperation,
        },
        properties::WindowSize,
    },
    terminal::ansi::AnsiParser,
    theme::{Alignment, ExecutionOutputBlockStyle, ExecutionStatusBlockStyle},
    ui::{
        execution::pty::{PtySnippetHandle, RunPtySnippetTrigger},
        separator::{RenderSeparator, SeparatorWidth},
    },
};
use std::{
    io::BufRead,
    iter,
    rc::Rc,
    sync::{Arc, Mutex},
};

const MINIMUM_SEPARATOR_WIDTH: u16 = 32;

#[derive(Default, Debug)]
enum State {
    #[default]
    Initial,
    Running(ExecutionHandle),
    Done,
}

#[derive(Debug)]
struct Inner {
    snippet: Snippet,
    executor: LanguageSnippetExecutor,
    output_lines: Vec<Line>,
    max_line_length: u16,
    process_status: Option<ProcessStatus>,
    state: State,
    policy: RenderAsyncStartPolicy,
}

#[derive(Debug)]
pub(crate) struct SnippetOutputOperation {
    default_colors: Colors,
    style: ExecutionOutputBlockStyle,
    block_length: u16,
    alignment: Alignment,
    handle: SnippetHandle,
    font_size: u8,
}

impl SnippetOutputOperation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        handle: SnippetHandle,
        default_colors: Colors,
        style: ExecutionOutputBlockStyle,
        block_length: u16,
        alignment: Alignment,
        font_size: u8,
    ) -> Self {
        let block_length = alignment.adjust_size(block_length);
        Self { default_colors, style, block_length, alignment, handle, font_size }
    }
}

impl AsRenderOperations for SnippetOutputOperation {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let inner = self.handle.0.lock().unwrap();
        if let State::Initial = inner.state {
            return Vec::new();
        }

        let mut operations = vec![];
        let block_colors = self.style.style.colors;
        if block_colors.background.is_some() {
            operations.push(RenderOperation::SetColors(block_colors));
        }

        if !inner.output_lines.is_empty() {
            let has_margin = match &self.alignment {
                Alignment::Left { margin } => !margin.is_empty(),
                Alignment::Right { margin } => !margin.is_empty(),
                Alignment::Center { minimum_margin, minimum_size } => !minimum_margin.is_empty() || minimum_size != &0,
            };
            let padding = self.style.padding;
            let block_length =
                if has_margin { self.block_length.max(inner.max_line_length) } else { inner.max_line_length };
            let vertical_padding = iter::repeat_n(" ", padding.vertical as usize).map(Line::from);
            let lines = vertical_padding.clone().chain(inner.output_lines.iter().cloned()).chain(vertical_padding);
            let style = TextStyle::default().size(self.font_size);
            for mut line in lines {
                line.apply_style(&style);
                let prefix = Text::new(" ".repeat(padding.horizontal as usize), style).into();
                operations.push(RenderOperation::RenderBlockLine(BlockLine {
                    prefix,
                    right_padding_length: padding.horizontal as u16,
                    repeat_prefix_on_wrap: false,
                    text: line.into(),
                    block_length,
                    alignment: self.alignment,
                    block_color: block_colors.background,
                }));
                operations.push(RenderOperation::RenderLineBreak);
            }
        }
        operations.extend([RenderOperation::SetColors(self.default_colors)]);
        operations
    }
}

struct OperationPollable {
    inner: Arc<Mutex<Inner>>,
    last_length: usize,
}

impl OperationPollable {
    fn try_start(&self, inner: &mut Inner) {
        // Don't run twice.
        if !matches!(inner.state, State::Initial) {
            return;
        }
        inner.state = match inner.executor.execute_async(&inner.snippet) {
            Ok(handle) => State::Running(handle),
            Err(e) => {
                inner.output_lines = vec![e.to_string().into()];
                State::Done
            }
        }
    }
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        let mut inner = self.inner.lock().unwrap();
        self.try_start(&mut inner);

        // At this point if we don't have a handle it's because we're done.
        let State::Running(handle) = &mut inner.state else {
            return PollableState::Done;
        };

        // Pull data out of the process' output and drop the handle state.
        let mut state = handle.state.lock().unwrap();
        let ExecutionState { output, status } = &mut *state;
        let status = *status;

        let modified = output.len() != self.last_length;
        let mut lines = Vec::new();
        for line in output.lines() {
            let mut line = line.expect("invalid utf8");
            if line.contains('\t') {
                line = line.replace('\t', "    ");
            }
            lines.push(line);
        }
        drop(state);

        let mut max_line_length = 0;
        let (lines, _) = AnsiParser::new(Default::default()).parse_lines(&lines);
        for line in &lines {
            let width = u16::try_from(line.width()).unwrap_or(u16::MAX);
            max_line_length = max_line_length.max(width);
        }

        let is_finished = status.is_finished();
        inner.process_status = Some(status);
        inner.output_lines = lines;
        inner.max_line_length = inner.max_line_length.max(max_line_length);
        if is_finished {
            inner.state = State::Done;
            PollableState::Done
        } else {
            match modified {
                true => PollableState::Modified,
                false => PollableState::Unmodified,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnippetHandle(Arc<Mutex<Inner>>);

impl SnippetHandle {
    pub(crate) fn new(code: Snippet, executor: LanguageSnippetExecutor, policy: RenderAsyncStartPolicy) -> Self {
        let inner = Inner {
            snippet: code,
            executor,
            process_status: Default::default(),
            output_lines: Default::default(),
            max_line_length: Default::default(),
            state: Default::default(),
            policy,
        };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub(crate) fn snippet(&self) -> Snippet {
        self.0.lock().unwrap().snippet.clone()
    }
}

#[derive(Debug)]
pub(crate) struct RunSnippetTrigger(Arc<Mutex<Inner>>);

impl RunSnippetTrigger {
    pub(crate) fn new(handle: SnippetHandle) -> Self {
        Self(handle.0)
    }
}

impl AsRenderOperations for RunSnippetTrigger {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        vec![]
    }
}

impl RenderAsync for RunSnippetTrigger {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable { inner: self.0.clone(), last_length: 0 })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        self.0.lock().unwrap().policy
    }
}

#[derive(Debug)]
pub(crate) struct ExecIndicatorStyle {
    pub(crate) theme: ExecutionStatusBlockStyle,
    pub(crate) block_length: u16,
    pub(crate) font_size: u8,
    pub(crate) alignment: Alignment,
}

#[derive(Clone, Debug)]
pub(crate) enum WrappedSnippetHandle {
    Normal(SnippetHandle),
    Pty(PtySnippetHandle),
}

impl WrappedSnippetHandle {
    pub(crate) fn process_status(&self) -> Option<ProcessStatus> {
        match self {
            Self::Normal(handle) => handle.0.lock().unwrap().process_status,
            Self::Pty(handle) => handle.process_status(),
        }
    }

    pub(crate) fn build_trigger(&self) -> Box<dyn RenderAsync> {
        match self.clone() {
            Self::Normal(handle) => Box::new(RunSnippetTrigger::new(handle)),
            Self::Pty(handle) => Box::new(RunPtySnippetTrigger::new(handle)),
        }
    }
}

impl From<SnippetHandle> for WrappedSnippetHandle {
    fn from(handle: SnippetHandle) -> Self {
        Self::Normal(handle)
    }
}

impl From<PtySnippetHandle> for WrappedSnippetHandle {
    fn from(handle: PtySnippetHandle) -> Self {
        Self::Pty(handle)
    }
}

#[derive(Debug)]
pub(crate) struct ExecIndicator {
    handle: WrappedSnippetHandle,
    separator_width: SeparatorWidth,
    theme: ExecutionStatusBlockStyle,
    font_size: u8,
}

impl ExecIndicator {
    pub(crate) fn new<T: Into<WrappedSnippetHandle>>(handle: T, style: ExecIndicatorStyle) -> Self {
        let ExecIndicatorStyle { theme, block_length, font_size, alignment } = style;
        let block_length = alignment.adjust_size(block_length);
        let separator_width = match &alignment {
            Alignment::Left { .. } | Alignment::Right { .. } => SeparatorWidth::FitToWindow,
            // We need a minimum here otherwise if the code/block length is too narrow, the separator is
            // word-wrapped and looks bad.
            Alignment::Center { .. } => {
                SeparatorWidth::Fixed(block_length.max(MINIMUM_SEPARATOR_WIDTH * font_size as u16))
            }
        };
        let handle = handle.into();
        Self { handle, separator_width, theme, font_size }
    }
}

impl AsRenderOperations for ExecIndicator {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let status = self.handle.process_status();
        let description = match status {
            Some(ProcessStatus::Running) => Text::new("running", self.theme.running_style),
            Some(ProcessStatus::Success) => Text::new("finished", self.theme.success_style),
            Some(ProcessStatus::Failure) => Text::new("finished with error", self.theme.failure_style),
            None => Text::new("not started", self.theme.not_started_style),
        };

        let heading = Line(vec![" [".into(), description.clone(), "] ".into()]);
        let separator = RenderSeparator::new(heading, self.separator_width, self.font_size);
        vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderDynamic(Rc::new(separator)),
            RenderOperation::RenderLineBreak,
        ]
    }
}

#[cfg(all(target_os = "linux", test))]
mod tests {
    use super::*;
    use crate::{
        code::{
            execute::SnippetExecutor,
            snippet::{SnippetAttributes, SnippetExecution, SnippetLanguage},
        },
        markdown::{
            elements::{Line, Text},
            text_style::Color,
        },
    };

    fn make_run_shell(code: &str) -> RunSnippetTrigger {
        let snippet = Snippet {
            contents: code.into(),
            language: SnippetLanguage::Bash,
            attributes: SnippetAttributes {
                execution: SnippetExecution::Exec(Default::default()),
                ..Default::default()
            },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let policy = RenderAsyncStartPolicy::OnDemand;
        let handle = SnippetHandle::new(snippet, executor, policy);
        RunSnippetTrigger::new(handle)
    }

    #[test]
    fn run_command() {
        let handle = make_run_shell("echo -e '\\033[1;31mhi mom'");
        let mut pollable = handle.pollable();
        // Run until done
        while let PollableState::Modified | PollableState::Unmodified = pollable.poll() {}

        // Expect to see the output lines
        let inner = handle.0.lock().unwrap();
        let line = Line::from(Text::new("hi mom", TextStyle::default().fg_color(Color::Red).bold()));
        assert_eq!(inner.output_lines, vec![line]);
    }

    #[test]
    fn multiple_pollables() {
        let handle = make_run_shell("echo -e '\\033[1;31mhi mom'");
        let mut main_pollable = handle.pollable();
        let mut pollable2 = handle.pollable();
        // Run until done
        while let PollableState::Modified | PollableState::Unmodified = main_pollable.poll() {}

        // Polling a pollable created early should return `Done` immediately
        assert_eq!(pollable2.poll(), PollableState::Done);

        // A new pollable should claim `Done` immediately
        let mut pollable3 = handle.pollable();
        assert_eq!(pollable3.poll(), PollableState::Done);
    }
}
