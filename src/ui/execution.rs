use super::separator::{RenderSeparator, SeparatorWidth};
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
            AsRenderOperations, BlockLine, ImageRenderProperties, RenderAsync, RenderAsyncState, RenderOperation,
        },
        properties::WindowSize,
    },
    terminal::{
        ansi::AnsiSplitter,
        image::{Image, printer::ImageRegistry},
        should_hide_cursor,
    },
    theme::{Alignment, ExecutionOutputBlockStyle, ExecutionStatusBlockStyle, Margin},
};
use crossterm::{
    ExecutableCommand, cursor,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use std::{
    cell::RefCell,
    io::{self, BufRead},
    mem,
    ops::{Deref, DerefMut},
    rc::Rc,
};

const MINIMUM_SEPARATOR_WIDTH: u16 = 32;

#[derive(Debug)]
struct RunSnippetOperationInner {
    handle: Option<ExecutionHandle>,
    output_lines: Vec<WeightedLine>,
    state: RenderAsyncState,
    max_line_length: u16,
    starting_style: TextStyle,
    last_length: usize,
}

#[derive(Debug)]
pub(crate) struct RunSnippetOperation {
    code: Snippet,
    executor: Rc<SnippetExecutor>,
    default_colors: Colors,
    block_colors: Colors,
    style: ExecutionStatusBlockStyle,
    block_length: u16,
    alignment: Alignment,
    inner: Rc<RefCell<RunSnippetOperationInner>>,
    state_description: RefCell<Text>,
    separator: DisplaySeparator,
    font_size: u8,
}

impl RunSnippetOperation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        code: Snippet,
        executor: Rc<SnippetExecutor>,
        default_colors: Colors,
        style: ExecutionOutputBlockStyle,
        block_length: u16,
        separator: DisplaySeparator,
        alignment: Alignment,
        font_size: u8,
    ) -> Self {
        let block_colors = style.style.colors;
        let status_colors = style.status.clone();
        let block_length = match &alignment {
            Alignment::Left { .. } | Alignment::Right { .. } => block_length,
            Alignment::Center { minimum_size, .. } => block_length.max(*minimum_size),
        };
        let inner = RunSnippetOperationInner {
            handle: None,
            output_lines: Vec::new(),
            state: RenderAsyncState::default(),
            max_line_length: 0,
            starting_style: TextStyle::default().size(font_size),
            last_length: 0,
        };
        Self {
            code,
            executor,
            default_colors,
            block_colors,
            style: status_colors,
            block_length,
            alignment,
            inner: Rc::new(RefCell::new(inner)),
            state_description: Text::new("not started", style.status.not_started_style).into(),
            separator,
            font_size,
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
        let inner = self.inner.borrow();
        let description = self.state_description.borrow();
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
        if matches!(inner.state, RenderAsyncState::NotStarted) {
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
    fn poll_state(&self) -> RenderAsyncState {
        let mut inner = self.inner.borrow_mut();
        let last_length = inner.last_length;
        if let Some(handle) = inner.handle.as_mut() {
            let mut state = handle.state.lock().unwrap();
            let ExecutionState { output, status } = &mut *state;
            *self.state_description.borrow_mut() = match status {
                ProcessStatus::Running => Text::new("running", self.style.running_style),
                ProcessStatus::Success => Text::new("finished", self.style.success_style),
                ProcessStatus::Failure => Text::new("finished with error", self.style.failure_style),
            };
            let modified = output.len() != last_length;
            let is_finished = status.is_finished();
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
            let (lines, style) = AnsiSplitter::new(inner.starting_style).split_lines(&lines);
            for line in &lines {
                let width = u16::try_from(line.width()).unwrap_or(u16::MAX);
                max_line_length = max_line_length.max(width);
            }
            inner.starting_style = style;
            if is_finished {
                inner.handle.take();
                inner.state = RenderAsyncState::JustFinishedRendering;
            } else {
                inner.state = RenderAsyncState::Rendering { modified };
            }
            inner.output_lines = lines;
            inner.max_line_length = inner.max_line_length.max(max_line_length);
        }
        inner.state.clone()
    }

    fn start_render(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if !matches!(inner.state, RenderAsyncState::NotStarted) {
            return false;
        }
        match self.executor.execute_async(&self.code) {
            Ok(handle) => {
                inner.handle = Some(handle);
                inner.state = RenderAsyncState::Rendering { modified: false };
                true
            }
            Err(e) => {
                inner.output_lines = vec![WeightedLine::from(e.to_string())];
                inner.state = RenderAsyncState::Rendered;
                true
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SnippetExecutionDisabledOperation {
    style: TextStyle,
    alignment: Alignment,
    started: RefCell<bool>,
}

impl SnippetExecutionDisabledOperation {
    pub(crate) fn new(style: TextStyle, alignment: Alignment) -> Self {
        Self { style, alignment, started: Default::default() }
    }
}

impl AsRenderOperations for SnippetExecutionDisabledOperation {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        if !*self.started.borrow() {
            return Vec::new();
        }
        vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderText {
                line: vec![Text::new("snippet execution is disabled", self.style)].into(),
                alignment: self.alignment,
            },
            RenderOperation::RenderLineBreak,
        ]
    }
}

impl RenderAsync for SnippetExecutionDisabledOperation {
    fn start_render(&self) -> bool {
        let was_started = mem::replace(&mut *self.started.borrow_mut(), true);
        !was_started
    }

    fn poll_state(&self) -> RenderAsyncState {
        RenderAsyncState::Rendered
    }
}

#[derive(Default, Clone)]
enum AcquireTerminalSnippetState {
    #[default]
    NotStarted,
    Success,
    Failure(Vec<String>),
}

impl std::fmt::Debug for AcquireTerminalSnippetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Success => write!(f, "Success"),
            Self::Failure(_) => write!(f, "Failure"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RunAcquireTerminalSnippet {
    snippet: Snippet,
    block_length: u16,
    executor: Rc<SnippetExecutor>,
    colors: ExecutionStatusBlockStyle,
    state: RefCell<AcquireTerminalSnippetState>,
    font_size: u8,
}

impl RunAcquireTerminalSnippet {
    pub(crate) fn new(
        snippet: Snippet,
        executor: Rc<SnippetExecutor>,
        colors: ExecutionStatusBlockStyle,
        block_length: u16,
        font_size: u8,
    ) -> Self {
        Self { snippet, block_length, executor, colors, state: Default::default(), font_size }
    }
}

impl RunAcquireTerminalSnippet {
    fn invoke(&self) -> Result<(), String> {
        let mut stdout = io::stdout();
        stdout
            .execute(terminal::LeaveAlternateScreen)
            .and_then(|_| disable_raw_mode())
            .map_err(|e| format!("failed to deinit terminal: {e}"))?;

        // save result for later, but first reinit the terminal
        let result = self.executor.execute_sync(&self.snippet).map_err(|e| format!("failed to run snippet: {e}"));

        stdout
            .execute(terminal::EnterAlternateScreen)
            .and_then(|_| enable_raw_mode())
            .map_err(|e| format!("failed to reinit terminal: {e}"))?;
        if should_hide_cursor() {
            stdout.execute(cursor::Hide).map_err(|e| e.to_string())?;
        }
        result
    }
}

impl AsRenderOperations for RunAcquireTerminalSnippet {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let state = self.state.borrow();
        let separator_text = match state.deref() {
            AcquireTerminalSnippetState::NotStarted => Text::new("not started", self.colors.not_started_style),
            AcquireTerminalSnippetState::Success => Text::new("finished", self.colors.success_style),
            AcquireTerminalSnippetState::Failure(_) => Text::new("finished with error", self.colors.failure_style),
        };

        let heading = Line(vec![" [".into(), separator_text, "] ".into()]);
        let separator_width = SeparatorWidth::Fixed(self.block_length.max(MINIMUM_SEPARATOR_WIDTH));
        let separator = RenderSeparator::new(heading, separator_width, self.font_size);
        let mut ops = vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderDynamic(Rc::new(separator)),
            RenderOperation::RenderLineBreak,
        ];
        if let AcquireTerminalSnippetState::Failure(lines) = state.deref() {
            ops.push(RenderOperation::RenderLineBreak);
            for line in lines {
                ops.extend([
                    RenderOperation::RenderText {
                        line: vec![Text::new(line, self.colors.failure_style)].into(),
                        alignment: Alignment::Left { margin: Margin::Percent(25) },
                    },
                    RenderOperation::RenderLineBreak,
                ]);
            }
        }
        ops
    }
}

impl RenderAsync for RunAcquireTerminalSnippet {
    fn start_render(&self) -> bool {
        if !matches!(*self.state.borrow(), AcquireTerminalSnippetState::NotStarted) {
            return false;
        }
        if let Err(e) = self.invoke() {
            let lines = e.lines().map(ToString::to_string).collect();
            *self.state.borrow_mut() = AcquireTerminalSnippetState::Failure(lines);
        } else {
            *self.state.borrow_mut() = AcquireTerminalSnippetState::Success;
        }
        true
    }

    fn poll_state(&self) -> RenderAsyncState {
        RenderAsyncState::Rendered
    }
}

#[derive(Debug)]
pub(crate) struct RunImageSnippet {
    snippet: Snippet,
    executor: Rc<SnippetExecutor>,
    state: RefCell<RunImageSnippetState>,
    image_registry: ImageRegistry,
    colors: ExecutionStatusBlockStyle,
}

impl RunImageSnippet {
    pub(crate) fn new(
        snippet: Snippet,
        executor: Rc<SnippetExecutor>,
        image_registry: ImageRegistry,
        colors: ExecutionStatusBlockStyle,
    ) -> Self {
        Self { snippet, executor, image_registry, colors, state: Default::default() }
    }

    fn load_image(&self, data: &[u8]) -> Result<Image, String> {
        let image = match image::load_from_memory(data) {
            Ok(image) => image,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        self.image_registry.register_image(image).map_err(|e| e.to_string())
    }
}

impl RenderAsync for RunImageSnippet {
    fn start_render(&self) -> bool {
        if !matches!(*self.state.borrow(), RunImageSnippetState::NotStarted) {
            return false;
        }
        let state = match self.executor.execute_async(&self.snippet) {
            Ok(handle) => RunImageSnippetState::Running(handle),
            Err(e) => RunImageSnippetState::Failure(e.to_string().lines().map(ToString::to_string).collect()),
        };
        *self.state.borrow_mut() = state;
        true
    }

    fn poll_state(&self) -> RenderAsyncState {
        let mut state = self.state.borrow_mut();
        match state.deref_mut() {
            RunImageSnippetState::NotStarted => RenderAsyncState::NotStarted,
            RunImageSnippetState::Running(handle) => {
                let mut inner = handle.state.lock().unwrap();
                match inner.status {
                    ProcessStatus::Running => RenderAsyncState::Rendering { modified: false },
                    ProcessStatus::Success => {
                        let data = mem::take(&mut inner.output);
                        drop(inner);

                        let image = match self.load_image(&data) {
                            Ok(image) => image,
                            Err(e) => {
                                *state = RunImageSnippetState::Failure(vec![e.to_string()]);
                                return RenderAsyncState::JustFinishedRendering;
                            }
                        };
                        *state = RunImageSnippetState::Success(image);
                        RenderAsyncState::JustFinishedRendering
                    }
                    ProcessStatus::Failure => {
                        let mut lines = Vec::new();
                        for line in inner.output.lines() {
                            lines.push(line.unwrap_or_else(|_| String::new()));
                        }
                        drop(inner);

                        *state = RunImageSnippetState::Failure(lines);
                        RenderAsyncState::JustFinishedRendering
                    }
                }
            }
            RunImageSnippetState::Success(_) | RunImageSnippetState::Failure(_) => RenderAsyncState::Rendered,
        }
    }
}

impl AsRenderOperations for RunImageSnippet {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let state = self.state.borrow();
        match state.deref() {
            RunImageSnippetState::NotStarted | RunImageSnippetState::Running(_) => vec![],
            RunImageSnippetState::Success(image) => {
                vec![RenderOperation::RenderImage(image.clone(), ImageRenderProperties::default())]
            }
            RunImageSnippetState::Failure(lines) => {
                let mut output = Vec::new();
                for line in lines {
                    output.extend([RenderOperation::RenderText {
                        line: vec![Text::new(line, self.colors.failure_style)].into(),
                        alignment: Alignment::Left { margin: Margin::Percent(25) },
                    }]);
                }
                output
            }
        }
    }
}

#[derive(Debug, Default)]
enum RunImageSnippetState {
    #[default]
    NotStarted,
    Running(ExecutionHandle),
    Success(Image),
    Failure(Vec<String>),
}
