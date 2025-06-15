use crate::{
    code::{
        execute::{ExecutionHandle, ExecutionState, LanguageSnippetExecutor, ProcessStatus},
        snippet::Snippet,
    },
    markdown::{
        elements::{Line, Text},
        text::WeightedLine,
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
    theme::{Alignment, ExecutionOutputBlockStyle, ExecutionStatusBlockStyle, PaddingRect},
    ui::separator::{RenderSeparator, SeparatorWidth},
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

#[derive(Debug, Default)]
struct Inner {
    output_lines: Vec<WeightedLine>,
    max_line_length: u16,
    process_status: Option<ProcessStatus>,
    state: State,
}

#[derive(Debug)]
pub(crate) struct RunSnippetOperation {
    code: Snippet,
    executor: LanguageSnippetExecutor,
    default_colors: Colors,
    block_colors: Colors,
    style: ExecutionStatusBlockStyle,
    block_length: u16,
    alignment: Alignment,
    inner: Arc<Mutex<Inner>>,
    separator: DisplaySeparator,
    font_size: u8,
    policy: RenderAsyncStartPolicy,
    padding: PaddingRect,
}

impl RunSnippetOperation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        code: Snippet,
        executor: LanguageSnippetExecutor,
        default_colors: Colors,
        style: ExecutionOutputBlockStyle,
        block_length: u16,
        separator: DisplaySeparator,
        alignment: Alignment,
        font_size: u8,
        policy: RenderAsyncStartPolicy,
        padding: PaddingRect,
    ) -> Self {
        let block_colors = style.style.colors;
        let status_colors = style.status.clone();
        let block_length = alignment.adjust_size(block_length);
        let inner = Inner::default();
        Self {
            code,
            executor,
            default_colors,
            block_colors,
            style: status_colors,
            block_length,
            alignment,
            inner: Arc::new(Mutex::new(inner)),
            separator,
            font_size,
            policy,
            padding,
        }
    }
}

#[derive(Debug)]
pub(crate) enum DisplaySeparator {
    On,
    Off,
}

impl AsRenderOperations for RunSnippetOperation {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let inner = self.inner.lock().unwrap();
        let description = match &inner.process_status {
            Some(ProcessStatus::Running) => Text::new("running", self.style.running_style),
            Some(ProcessStatus::Success) => Text::new("finished", self.style.success_style),
            Some(ProcessStatus::Failure) => Text::new("finished with error", self.style.failure_style),
            None => Text::new("not started", self.style.not_started_style),
        };
        let mut operations = match self.separator {
            DisplaySeparator::On => {
                let heading = Line(vec![" [".into(), description.clone(), "] ".into()]);
                let separator_width = match &self.alignment {
                    Alignment::Left { .. } | Alignment::Right { .. } => SeparatorWidth::FitToWindow,
                    // We need a minimum here otherwise if the code/block length is too narrow, the separator is
                    // word-wrapped and looks bad.
                    Alignment::Center { .. } => SeparatorWidth::Fixed(self.block_length.max(MINIMUM_SEPARATOR_WIDTH)),
                };
                let separator = RenderSeparator::new(heading, separator_width, self.font_size);
                vec![
                    RenderOperation::RenderLineBreak,
                    RenderOperation::RenderDynamic(Rc::new(separator)),
                    RenderOperation::RenderLineBreak,
                ]
            }
            DisplaySeparator::Off => vec![],
        };
        if let State::Initial = inner.state {
            return operations;
        }
        operations.push(RenderOperation::RenderLineBreak);

        if self.block_colors.background.is_some() {
            operations.push(RenderOperation::SetColors(self.block_colors));
        }

        if !inner.output_lines.is_empty() {
            let has_margin = match &self.alignment {
                Alignment::Left { margin } => !margin.is_empty(),
                Alignment::Right { margin } => !margin.is_empty(),
                Alignment::Center { minimum_margin, minimum_size } => !minimum_margin.is_empty() || minimum_size != &0,
            };
            let block_length =
                if has_margin { self.block_length.max(inner.max_line_length) } else { inner.max_line_length };
            let vertical_padding = iter::repeat_n(" ", self.padding.vertical as usize).map(WeightedLine::from);
            let lines = vertical_padding.clone().chain(inner.output_lines.iter().cloned()).chain(vertical_padding);
            for line in lines {
                operations.push(RenderOperation::RenderBlockLine(BlockLine {
                    prefix: " ".repeat(self.padding.horizontal as usize).into(),
                    right_padding_length: self.padding.horizontal as u16,
                    repeat_prefix_on_wrap: false,
                    text: line,
                    block_length,
                    alignment: self.alignment,
                    block_color: self.block_colors.background,
                }));
                operations.push(RenderOperation::RenderLineBreak);
            }
        }
        operations.push(RenderOperation::SetColors(self.default_colors));
        operations
    }
}

impl RenderAsync for RunSnippetOperation {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable {
            inner: self.inner.clone(),
            executor: self.executor.clone(),
            code: self.code.clone(),
            last_length: 0,
            style: TextStyle::default().size(self.font_size),
        })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        self.policy
    }
}

struct OperationPollable {
    inner: Arc<Mutex<Inner>>,
    executor: LanguageSnippetExecutor,
    code: Snippet,
    last_length: usize,
    style: TextStyle,
}

impl OperationPollable {
    fn try_start(&self, inner: &mut Inner) {
        // Don't run twice.
        if !matches!(inner.state, State::Initial) {
            return;
        }
        inner.state = match self.executor.execute_async(&self.code) {
            Ok(handle) => State::Running(handle),
            Err(e) => {
                inner.output_lines = vec![WeightedLine::from(e.to_string())];
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
        let status = status.clone();

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
        let (lines, _) = AnsiParser::new(self.style).parse_lines(&lines);
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

#[cfg(all(target_os = "linux", test))]
mod tests {
    use super::*;
    use crate::{
        code::{
            execute::SnippetExecutor,
            snippet::{SnippetAttributes, SnippetExec, SnippetLanguage},
        },
        markdown::text_style::Color,
    };

    fn make_run_shell(code: &str) -> RunSnippetOperation {
        let snippet = Snippet {
            contents: code.into(),
            language: SnippetLanguage::Bash,
            attributes: SnippetAttributes { execution: SnippetExec::Exec(Default::default()), ..Default::default() },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let default_colors = Default::default();
        let style = ExecutionOutputBlockStyle::default();
        let block_length = 0;
        let separator = DisplaySeparator::On;
        let alignment = Default::default();
        let font_size = 1;
        let policy = RenderAsyncStartPolicy::OnDemand;
        RunSnippetOperation::new(
            snippet,
            executor,
            default_colors,
            style,
            block_length,
            separator,
            alignment,
            font_size,
            policy,
            Default::default(),
        )
    }

    #[test]
    fn run_command() {
        let operation = make_run_shell("echo -e '\\033[1;31mhi mom'");
        let mut pollable = operation.pollable();
        // Run until done
        while let PollableState::Modified | PollableState::Unmodified = pollable.poll() {}

        // Expect to see the output lines
        let inner = operation.inner.lock().unwrap();
        let line = Line::from(Text::new("hi mom", TextStyle::default().fg_color(Color::Red).bold()));
        assert_eq!(inner.output_lines, vec![line.into()]);
    }

    #[test]
    fn multiple_pollables() {
        let operation = make_run_shell("echo -e '\\033[1;31mhi mom'");
        let mut main_pollable = operation.pollable();
        let mut pollable2 = operation.pollable();
        // Run until done
        while let PollableState::Modified | PollableState::Unmodified = main_pollable.poll() {}

        // Polling a pollable created early should return `Done` immediately
        assert_eq!(pollable2.poll(), PollableState::Done);

        // A new pollable should claim `Done` immediately
        let mut pollable3 = operation.pollable();
        assert_eq!(pollable3.poll(), PollableState::Done);
    }
}
