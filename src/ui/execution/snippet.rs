use crate::{
    code::{
        execute::{ExecutionHandle, ExecutionState, ProcessStatus, SnippetExecutor},
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
    terminal::ansi::AnsiSplitter,
    theme::{Alignment, ExecutionOutputBlockStyle, ExecutionStatusBlockStyle},
    ui::separator::{RenderSeparator, SeparatorWidth},
};
use std::{
    io::BufRead,
    rc::Rc,
    sync::{Arc, Mutex},
};

const MINIMUM_SEPARATOR_WIDTH: u16 = 32;

#[derive(Debug)]
struct Inner {
    output_lines: Vec<WeightedLine>,
    max_line_length: u16,
    process_status: Option<ProcessStatus>,
    started: bool,
}

#[derive(Debug)]
pub(crate) struct RunSnippetOperation {
    code: Snippet,
    executor: Arc<SnippetExecutor>,
    default_colors: Colors,
    block_colors: Colors,
    style: ExecutionStatusBlockStyle,
    block_length: u16,
    alignment: Alignment,
    inner: Arc<Mutex<Inner>>,
    separator: DisplaySeparator,
    font_size: u8,
    policy: RenderAsyncStartPolicy,
}

impl RunSnippetOperation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        code: Snippet,
        executor: Arc<SnippetExecutor>,
        default_colors: Colors,
        style: ExecutionOutputBlockStyle,
        block_length: u16,
        separator: DisplaySeparator,
        alignment: Alignment,
        font_size: u8,
        policy: RenderAsyncStartPolicy,
    ) -> Self {
        let block_colors = style.style.colors;
        let status_colors = style.status.clone();
        let block_length = alignment.adjust_size(block_length);
        let inner = Inner { output_lines: Vec::new(), max_line_length: 0, process_status: None, started: false };
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
        if !inner.started {
            return operations;
        }
        operations.push(RenderOperation::RenderLineBreak);

        if self.block_colors.background.is_some() {
            operations.push(RenderOperation::SetColors(self.block_colors));
        }

        let has_margin = match &self.alignment {
            Alignment::Left { margin } => !margin.is_empty(),
            Alignment::Right { margin } => !margin.is_empty(),
            Alignment::Center { minimum_margin, minimum_size } => !minimum_margin.is_empty() || minimum_size != &0,
        };
        let block_length =
            if has_margin { self.block_length.max(inner.max_line_length) } else { inner.max_line_length };
        for line in &inner.output_lines {
            operations.push(RenderOperation::RenderBlockLine(BlockLine {
                prefix: "".into(),
                right_padding_length: 0,
                repeat_prefix_on_wrap: false,
                text: line.clone(),
                block_length,
                alignment: self.alignment,
                block_color: self.block_colors.background,
            }));
            operations.push(RenderOperation::RenderLineBreak);
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
            handle: None,
            last_length: 0,
            starting_style: TextStyle::default().size(self.font_size),
        })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        self.policy
    }
}

struct OperationPollable {
    inner: Arc<Mutex<Inner>>,
    executor: Arc<SnippetExecutor>,
    code: Snippet,
    handle: Option<ExecutionHandle>,
    last_length: usize,
    starting_style: TextStyle,
}

impl OperationPollable {
    fn try_start(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.started {
            return;
        }
        inner.started = true;
        match self.executor.execute_async(&self.code) {
            Ok(handle) => {
                self.handle = Some(handle);
            }
            Err(e) => {
                inner.output_lines = vec![WeightedLine::from(e.to_string())];
            }
        }
    }
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        self.try_start();

        // At this point if we don't have a handle it's because we're done.
        let Some(handle) = self.handle.as_mut() else { return PollableState::Done };

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
        let (lines, style) = AnsiSplitter::new(self.starting_style).split_lines(&lines);
        for line in &lines {
            let width = u16::try_from(line.width()).unwrap_or(u16::MAX);
            max_line_length = max_line_length.max(width);
        }

        let mut inner = self.inner.lock().unwrap();
        let is_finished = status.is_finished();
        inner.process_status = Some(status);
        inner.output_lines = lines;
        inner.max_line_length = inner.max_line_length.max(max_line_length);
        if is_finished {
            self.handle.take();
            PollableState::Done
        } else {
            // Save the style so we continue with it next time
            self.starting_style = style;
            match modified {
                true => PollableState::Modified,
                false => PollableState::Unmodified,
            }
        }
    }
}
