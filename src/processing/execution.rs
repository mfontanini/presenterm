use super::separator::RenderSeparator;
use crate::{
    execute::{CodeExecutor, ExecutionHandle, ExecutionState, ProcessStatus},
    markdown::elements::{Code, Text, TextBlock},
    presentation::{AsRenderOperations, PreformattedLine, RenderAsync, RenderAsyncState, RenderOperation},
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::ExecutionStatusBlockStyle,
};
use itertools::Itertools;
use std::{cell::RefCell, rc::Rc};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
struct RunCodeOperationInner {
    handle: Option<ExecutionHandle>,
    output_lines: Vec<String>,
    state: RenderAsyncState,
}

#[derive(Debug)]
pub(crate) struct RunCodeOperation {
    code: Code,
    executor: Rc<CodeExecutor>,
    default_colors: Colors,
    block_colors: Colors,
    status_colors: ExecutionStatusBlockStyle,
    inner: Rc<RefCell<RunCodeOperationInner>>,
    state_description: RefCell<Text>,
}

impl RunCodeOperation {
    pub(crate) fn new(
        code: Code,
        executor: Rc<CodeExecutor>,
        default_colors: Colors,
        block_colors: Colors,
        status_colors: ExecutionStatusBlockStyle,
    ) -> Self {
        let inner =
            RunCodeOperationInner { handle: None, output_lines: Vec::new(), state: RenderAsyncState::default() };
        let running_colors = status_colors.running.clone();
        Self {
            code,
            executor,
            default_colors,
            block_colors,
            status_colors,
            inner: Rc::new(RefCell::new(inner)),
            state_description: Text::new("running", TextStyle::default().colors(running_colors)).into(),
        }
    }

    fn render_line(&self, mut line: String) -> RenderOperation {
        if line.contains('\t') {
            line = line.replace('\t', "    ");
        }
        let line_len = line.width() as u16;
        RenderOperation::RenderPreformattedLine(PreformattedLine {
            text: line,
            unformatted_length: line_len,
            block_length: line_len,
            alignment: Default::default(),
        })
    }
}

impl AsRenderOperations for RunCodeOperation {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let inner = self.inner.borrow();
        if matches!(inner.state, RenderAsyncState::NotStarted) {
            return Vec::new();
        }
        let description = self.state_description.borrow();
        let heading = TextBlock(vec![" [".into(), description.clone(), "] ".into()]);
        let separator = RenderSeparator::new(heading);
        let mut operations = vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderDynamic(Rc::new(separator)),
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
            RenderOperation::SetColors(self.block_colors.clone()),
        ];

        for line in &inner.output_lines {
            let chunks = line.chars().chunks(dimensions.columns as usize);
            for chunk in &chunks {
                operations.push(self.render_line(chunk.collect()));
                operations.push(RenderOperation::RenderLineBreak);
            }
        }
        operations.push(RenderOperation::SetColors(self.default_colors.clone()));
        operations
    }

    fn diffable_content(&self) -> Option<&str> {
        None
    }
}

impl RenderAsync for RunCodeOperation {
    fn poll_state(&self) -> RenderAsyncState {
        let mut inner = self.inner.borrow_mut();
        if let Some(handle) = inner.handle.as_mut() {
            let state = handle.state();
            let ExecutionState { output, status } = state;
            *self.state_description.borrow_mut() = match status {
                ProcessStatus::Running => {
                    Text::new("running", TextStyle::default().colors(self.status_colors.running.clone()))
                }
                ProcessStatus::Success => {
                    Text::new("finished", TextStyle::default().colors(self.status_colors.success.clone()))
                }
                ProcessStatus::Failure => {
                    Text::new("finished with error", TextStyle::default().colors(self.status_colors.failure.clone()))
                }
            };
            let modified = inner.output_lines.len() != output.len();
            if status.is_finished() {
                inner.handle.take();
                inner.state = RenderAsyncState::JustFinishedRendering;
            } else {
                inner.state = RenderAsyncState::Rendering { modified };
            }
            inner.output_lines = output;
        }
        inner.state.clone()
    }

    fn start_render(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if !matches!(inner.state, RenderAsyncState::NotStarted) {
            return false;
        }
        match self.executor.execute(&self.code) {
            Ok(handle) => {
                inner.handle = Some(handle);
                inner.state = RenderAsyncState::Rendering { modified: false };
                true
            }
            Err(e) => {
                inner.output_lines = vec![e.to_string()];
                inner.state = RenderAsyncState::Rendered;
                true
            }
        }
    }
}
