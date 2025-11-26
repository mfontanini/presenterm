use crate::{
    code::{
        execute::{LanguageSnippetExecutor, ProcessStatus, PtySnippetContext},
        snippet::{PtyArgs, Snippet},
    },
    markdown::{
        elements::{Line, Text},
        text_style::{Color, TextStyle},
    },
    render::{
        operation::{
            AsRenderOperations, BlockLine, Pollable, PollableState, RenderAsync, RenderAsyncStartPolicy,
            RenderOperation,
        },
        properties::WindowSize,
    },
    theme::{Alignment, PtyOutputBlockStyle},
};
use portable_pty::{MasterPty, PtySize, native_pty_system};
use std::{
    fmt, io, iter, mem,
    sync::{Arc, Mutex},
    thread,
};
use unicode_width::UnicodeWidthStr;

const BOTTOM_MARGIN_RATIO: f64 = 0.2;
const BOTTOM_MINIMUM_MARGIN: u16 = 7;
const DEFAULT_COLUMNS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

#[derive(Default, Debug)]
enum State {
    #[default]
    Initial,
    Running {
        pty: PtyMaster,
        dirty: bool,
    },
    ProcessTerminated(ProcessStatus),
    Done(ProcessStatus),
}

struct Inner {
    snippet: Snippet,
    executor: LanguageSnippetExecutor,
    parser: vt100::Parser,
    expected_size: WindowSize,
    actual_size: WindowSize,
    update_size: bool,
    standby: bool,
    policy: RenderAsyncStartPolicy,
    state: State,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("snippet", &self.snippet)
            .field("executor", &self.executor)
            .field("expected_size", &self.expected_size)
            .field("actual_size", &self.actual_size)
            .field("update_size", &self.update_size)
            .field("standby", &self.standby)
            .field("parser", &"...")
            .field("policy", &self.policy)
            .field("state", &"...")
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct PtySnippetOutputOperation {
    handle: PtySnippetHandle,
    style: PtyOutputBlockStyle,
    font_size: u8,
}

impl PtySnippetOutputOperation {
    pub(crate) fn new(handle: PtySnippetHandle, style: PtyOutputBlockStyle, font_size: u8) -> Self {
        Self { handle, style, font_size }
    }

    fn standby_row(&self, row: u16, dimensions: &WindowSize) -> Line {
        let lines = self.style.standby.as_lines();
        let start_index = (dimensions.rows / 2).saturating_sub(lines.len() as u16 / 2);
        if row < start_index || row >= start_index + lines.len() as u16 {
            "".into()
        } else {
            let index = (row - start_index) as usize;
            let padding = usize::from(dimensions.columns / 2).saturating_sub(lines[index].width() / 2);
            let line: String = iter::repeat_n(' ', padding).chain(lines[index].chars()).collect();
            Text::new(line, self.style.style).into()
        }
    }
}

impl AsRenderOperations for PtySnippetOutputOperation {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let mut inner = self.handle.0.lock().unwrap();
        let vertical_padding = ((dimensions.rows as f64 * BOTTOM_MARGIN_RATIO) as u16).max(BOTTOM_MINIMUM_MARGIN);
        let dimensions = dimensions
            .shrink_rows(vertical_padding / self.font_size as u16)
            .shrink_columns(dimensions.columns - dimensions.columns / self.font_size as u16);

        if inner.update_size && inner.expected_size != dimensions && dimensions.rows > 0 {
            inner.expected_size = dimensions;
            inner.parser.screen_mut().set_size(dimensions.rows, dimensions.columns);
        }
        if matches!(inner.state, State::Initial) {
            let mut operations = Vec::new();
            if inner.standby {
                let dimensions = inner.expected_size;
                for row in 0..dimensions.rows {
                    let line = self.standby_row(row, &dimensions);
                    operations.extend([
                        RenderOperation::RenderBlockLine(BlockLine {
                            prefix: "".into(),
                            right_padding_length: 0,
                            repeat_prefix_on_wrap: false,
                            text: line.into(),
                            block_length: dimensions.columns,
                            block_color: self.style.style.colors.background,
                            alignment: Alignment::Center {
                                minimum_margin: Default::default(),
                                minimum_size: Default::default(),
                            },
                        }),
                        RenderOperation::RenderLineBreak,
                    ]);
                }
            }
            return operations;
        }

        let screen = inner.parser.screen();
        let (rows, columns) = screen.size();
        let mut operations = vec![];

        for row in 0..rows {
            let mut line = Vec::new();
            let mut current_text = String::new();
            let mut current_style = TextStyle::default();
            for column in 0..columns {
                let cell = screen.cell(row, column).expect("no cell");
                let mut style = TextStyle::from(cell).size(self.font_size);
                if style.colors.foreground.is_none() {
                    style.colors.foreground = self.style.style.colors.foreground;
                }
                if style.colors.background.is_none() {
                    style.colors.background = self.style.style.colors.background;
                }
                let contents = cell.contents();
                if current_style != style && !current_text.is_empty() {
                    line.push(Text::new(mem::take(&mut current_text), current_style));
                }
                current_style = style;
                if contents.is_empty() {
                    current_text.push(' ');
                } else {
                    current_text.push_str(contents);
                }
            }
            if !current_text.is_empty() {
                line.push(Text::new(current_text, current_style));
            }
            operations.extend([
                RenderOperation::RenderBlockLine(BlockLine {
                    prefix: "".into(),
                    right_padding_length: 0,
                    repeat_prefix_on_wrap: false,
                    text: line.into(),
                    block_length: columns,
                    block_color: None,
                    alignment: Alignment::Center {
                        minimum_margin: Default::default(),
                        minimum_size: Default::default(),
                    },
                }),
                RenderOperation::RenderLineBreak,
            ]);
        }
        operations
    }
}

impl RenderAsync for PtySnippetOutputOperation {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable { handle: self.handle.clone() })
    }
}

#[derive(Debug)]
struct OperationPollable {
    handle: PtySnippetHandle,
}

impl OperationPollable {
    fn spawn(ctx: PtySnippetContext, dimensions: WindowSize, handle: PtySnippetHandle) -> anyhow::Result<PtyMaster> {
        let pty_system = native_pty_system();
        let pty_size = PtySize {
            rows: dimensions.rows,
            cols: dimensions.columns,
            pixel_width: dimensions.pixels_per_column() as u16,
            pixel_height: dimensions.pixels_per_row() as u16,
        };
        let pair = pty_system.openpty(pty_size)?;
        pair.slave.spawn_command(ctx.command.clone())?;
        PtyMaster::new(pair.master, handle, ctx)
    }
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        let mut inner = self.handle.0.lock().unwrap();
        let expected_size = inner.expected_size;
        let actual_size = inner.actual_size;
        inner.actual_size = expected_size;
        match &mut inner.state {
            State::Initial => match inner.executor.pty_execution_context(&inner.snippet) {
                Ok(ctx) => match Self::spawn(ctx, expected_size, self.handle.clone()) {
                    Ok(pty) => {
                        inner.state = State::Running { pty, dirty: true };
                        PollableState::Modified
                    }
                    Err(e) => {
                        inner.state = State::Done(ProcessStatus::Failure);
                        PollableState::Failed { error: format!("failed to run script: {e}") }
                    }
                },
                Err(e) => {
                    inner.state = State::Done(ProcessStatus::Failure);
                    PollableState::Failed { error: format!("failed to run script: {e}") }
                }
            },
            State::Running { dirty, pty } => {
                if actual_size != expected_size {
                    let size = PtySize {
                        rows: expected_size.rows,
                        cols: expected_size.columns,
                        pixel_width: 0,
                        pixel_height: 0,
                    };
                    let _ = pty._master.resize(size);
                }

                if mem::take(dirty) { PollableState::Modified } else { PollableState::Unmodified }
            }
            State::ProcessTerminated(status) => {
                inner.state = State::Done(*status);
                PollableState::Modified
            }
            _ => PollableState::Unmodified,
        }
    }
}

pub(crate) struct PtyMaster {
    _master: Box<dyn MasterPty>,
    _ctx: PtySnippetContext,
}

impl PtyMaster {
    fn new(master: Box<dyn MasterPty>, handle: PtySnippetHandle, ctx: PtySnippetContext) -> anyhow::Result<Self> {
        let reader = master.try_clone_reader()?;
        thread::spawn(|| process_output(reader, handle));
        Ok(Self { _master: master, _ctx: ctx })
    }
}

impl fmt::Debug for PtyMaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PtyMaster").field("master", &"...").finish()
    }
}

fn process_output(mut reader: Box<dyn io::Read>, handle: PtySnippetHandle) {
    let mut input_buffer = [0; 1024];
    let status = loop {
        let Ok(bytes_read) = reader.read(&mut input_buffer) else {
            break ProcessStatus::Failure;
        };
        if bytes_read == 0 {
            break ProcessStatus::Success;
        }
        let bytes = &input_buffer[..bytes_read];
        let mut inner = handle.0.lock().unwrap();
        inner.parser.process(bytes);
        if let State::Running { dirty, .. } = &mut inner.state {
            *dirty = true;
        };
    };
    handle.0.lock().unwrap().state = State::ProcessTerminated(status);
}

impl From<&vt100::Cell> for TextStyle {
    fn from(cell: &vt100::Cell) -> Self {
        let mut style = TextStyle::default();
        if cell.bold() {
            style = style.bold();
        }
        if cell.italic() {
            style = style.italics();
        }
        if cell.underline() {
            style = style.underlined();
        }
        style.colors.foreground = parse_color(cell.fgcolor());
        style.colors.background = parse_color(cell.bgcolor());
        style
    }
}

fn parse_color(color: vt100::Color) -> Option<Color> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(value) => Color::from_8bit(value),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb { r, g, b }),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PtySnippetHandle(Arc<Mutex<Inner>>);

impl PtySnippetHandle {
    pub(crate) fn new(
        snippet: Snippet,
        executor: LanguageSnippetExecutor,
        policy: RenderAsyncStartPolicy,
        args: PtyArgs,
    ) -> Self {
        let expected_size = WindowSize {
            columns: args.columns.unwrap_or(DEFAULT_COLUMNS),
            rows: args.rows.unwrap_or(DEFAULT_ROWS),
            height: 0,
            width: 0,
        };
        let update_size = args.columns.is_none() || args.rows.is_none();
        let parser = vt100::Parser::new(expected_size.rows, expected_size.columns, 1000);
        let inner = Inner {
            snippet,
            executor,
            parser,
            expected_size,
            actual_size: expected_size,
            update_size,
            standby: args.standby,
            state: Default::default(),
            policy,
        };
        Self(Arc::new(Mutex::new(inner)))
    }

    pub(crate) fn snippet(&self) -> Snippet {
        self.0.lock().unwrap().snippet.clone()
    }

    pub(crate) fn process_status(&self) -> Option<ProcessStatus> {
        match &self.0.lock().unwrap().state {
            State::Initial => None,
            State::Running { .. } => Some(ProcessStatus::Running),
            State::ProcessTerminated(status) | State::Done(status) => Some(*status),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RunPtySnippetTrigger(PtySnippetHandle);

impl RunPtySnippetTrigger {
    pub(crate) fn new(handle: PtySnippetHandle) -> Self {
        Self(handle)
    }
}

impl AsRenderOperations for RunPtySnippetTrigger {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        vec![]
    }
}

impl RenderAsync for RunPtySnippetTrigger {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable { handle: self.0.clone() })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        self.0.0.lock().unwrap().policy
    }
}
