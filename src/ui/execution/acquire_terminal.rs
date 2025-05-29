use crate::{
    code::{execute::LanguageSnippetExecutor, snippet::Snippet},
    markdown::elements::{Line, Text},
    render::{
        operation::{AsRenderOperations, Pollable, PollableState, RenderAsync, RenderOperation, RenderTextProperties},
        properties::WindowSize,
    },
    terminal::should_hide_cursor,
    theme::{Alignment, ExecutionStatusBlockStyle, Margin},
    ui::separator::{RenderSeparator, SeparatorWidth},
};
use crossterm::{
    ExecutableCommand, cursor,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self},
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
};

const MINIMUM_SEPARATOR_WIDTH: u16 = 32;

#[derive(Debug)]
pub(crate) struct RunAcquireTerminalSnippet {
    snippet: Snippet,
    block_length: u16,
    executor: LanguageSnippetExecutor,
    colors: ExecutionStatusBlockStyle,
    state: Arc<Mutex<State>>,
    font_size: u8,
}

impl RunAcquireTerminalSnippet {
    pub(crate) fn new(
        snippet: Snippet,
        executor: LanguageSnippetExecutor,
        colors: ExecutionStatusBlockStyle,
        block_length: u16,
        font_size: u8,
    ) -> Self {
        Self { snippet, block_length, executor, colors, state: Default::default(), font_size }
    }

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
        let state = self.state.lock().unwrap();
        let separator_text = match state.deref() {
            State::NotStarted => Text::new("not started", self.colors.not_started_style),
            State::Success => Text::new("finished", self.colors.success_style),
            State::Failure(_) => Text::new("finished with error", self.colors.failure_style),
        };

        let heading = Line(vec![" [".into(), separator_text, "] ".into()]);
        let separator_width = SeparatorWidth::Fixed(self.block_length.max(MINIMUM_SEPARATOR_WIDTH));
        let separator = RenderSeparator::new(heading, separator_width, self.font_size);
        let mut ops = vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderDynamic(Rc::new(separator)),
            RenderOperation::RenderLineBreak,
        ];
        if let State::Failure(lines) = state.deref() {
            ops.push(RenderOperation::RenderLineBreak);
            for line in lines {
                ops.extend([
                    RenderOperation::RenderText {
                        line: vec![Text::new(line, self.colors.failure_style)].into(),
                        properties: RenderTextProperties {
                            alignment: Alignment::Left { margin: Margin::Percent(25) },
                            ..Default::default()
                        },
                    },
                    RenderOperation::RenderLineBreak,
                ]);
            }
        }
        ops
    }
}

impl RenderAsync for RunAcquireTerminalSnippet {
    fn pollable(&self) -> Box<dyn Pollable> {
        // Run within this method because we need to release/acquire the raw terminal in the main
        // thread.
        let mut state = self.state.lock().unwrap();
        if matches!(*state, State::NotStarted) {
            if let Err(e) = self.invoke() {
                let lines = e.lines().map(ToString::to_string).collect();
                *state = State::Failure(lines);
            } else {
                *state = State::Success;
            }
        }
        Box::new(OperationPollable)
    }
}

#[derive(Default, Clone)]
enum State {
    #[default]
    NotStarted,
    Success,
    Failure(Vec<String>),
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Success => write!(f, "Success"),
            Self::Failure(_) => write!(f, "Failure"),
        }
    }
}

struct OperationPollable;

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        PollableState::Done
    }
}
