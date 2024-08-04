use super::separator::{RenderSeparator, SeparatorWidth};
use crate::{
    ansi::AnsiSplitter,
    execute::{ExecutionHandle, ExecutionState, ProcessStatus, SnippetExecutor},
    markdown::{
        elements::{Text, TextBlock},
        text::WeightedTextBlock,
    },
    presentation::{AsRenderOperations, BlockLine, RenderAsync, RenderAsyncState, RenderOperation},
    processing::code::Snippet,
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::{Alignment, ExecutionStatusBlockStyle},
    PresentationTheme,
};
use std::{cell::RefCell, mem, rc::Rc};

#[derive(Debug)]
struct RunSnippetOperationInner {
    handle: Option<ExecutionHandle>,
    output_lines: Vec<WeightedTextBlock>,
    state: RenderAsyncState,
    max_line_length: u16,
    starting_style: TextStyle,
}

#[derive(Debug)]
pub(crate) struct RunSnippetOperation {
    code: Snippet,
    executor: Rc<SnippetExecutor>,
    default_colors: Colors,
    block_colors: Colors,
    status_colors: ExecutionStatusBlockStyle,
    block_length: u16,
    alignment: Alignment,
    inner: Rc<RefCell<RunSnippetOperationInner>>,
    state_description: RefCell<Text>,
}

impl RunSnippetOperation {
    pub(crate) fn new(
        code: Snippet,
        executor: Rc<SnippetExecutor>,
        theme: &PresentationTheme,
        block_length: u16,
    ) -> Self {
        let default_colors = theme.default_style.colors;
        let block_colors = theme.execution_output.colors;
        let status_colors = theme.execution_output.status.clone();
        let not_started_colors = status_colors.not_started;
        let alignment = theme.code.alignment.clone().unwrap_or_default();
        let block_length = match &alignment {
            Alignment::Left { .. } | Alignment::Right { .. } => block_length,
            Alignment::Center { minimum_size, .. } => block_length.max(*minimum_size),
        };
        let inner = RunSnippetOperationInner {
            handle: None,
            output_lines: Vec::new(),
            state: RenderAsyncState::default(),
            max_line_length: 0,
            starting_style: TextStyle::default(),
        };
        Self {
            code,
            executor,
            default_colors,
            block_colors,
            status_colors,
            block_length,
            alignment,
            inner: Rc::new(RefCell::new(inner)),
            state_description: Text::new("not started", TextStyle::default().colors(not_started_colors)).into(),
        }
    }
}

impl AsRenderOperations for RunSnippetOperation {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let inner = self.inner.borrow();
        let description = self.state_description.borrow();
        let heading = TextBlock(vec![" [".into(), description.clone(), "] ".into()]);
        let separator_width = match &self.alignment {
            Alignment::Left { .. } | Alignment::Right { .. } => SeparatorWidth::FitToWindow,
            Alignment::Center { .. } => SeparatorWidth::Fixed(self.block_length),
        };
        let separator = RenderSeparator::new(heading, separator_width);
        let mut operations = vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderDynamic(Rc::new(separator)),
            RenderOperation::RenderLineBreak,
        ];
        if matches!(inner.state, RenderAsyncState::NotStarted) {
            return operations;
        }
        operations.extend([RenderOperation::RenderLineBreak, RenderOperation::SetColors(self.block_colors)]);

        let block_length = self.block_length.max(inner.max_line_length.saturating_add(1));
        for line in &inner.output_lines {
            operations.push(RenderOperation::RenderBlockLine(BlockLine {
                prefix: "".into(),
                text: line.clone(),
                block_length,
                alignment: self.alignment.clone(),
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
        if let Some(handle) = inner.handle.as_mut() {
            let mut state = handle.state.lock().unwrap();
            let ExecutionState { output, status } = &mut *state;
            *self.state_description.borrow_mut() = match status {
                ProcessStatus::Running => Text::new("running", TextStyle::default().colors(self.status_colors.running)),
                ProcessStatus::Success => {
                    Text::new("finished", TextStyle::default().colors(self.status_colors.success))
                }
                ProcessStatus::Failure => {
                    Text::new("finished with error", TextStyle::default().colors(self.status_colors.failure))
                }
            };
            let new_lines = mem::take(output);
            let modified = !new_lines.is_empty();
            let is_finished = status.is_finished();
            drop(state);

            let mut max_line_length = 0;
            let (new_lines, style) = AnsiSplitter::new(inner.starting_style).split_lines(&new_lines);
            for line in &new_lines {
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
            inner.output_lines.extend(new_lines);
            inner.max_line_length = inner.max_line_length.max(max_line_length);
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
                inner.output_lines = vec![WeightedTextBlock::from(e.to_string())];
                inner.state = RenderAsyncState::Rendered;
                true
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SnippetExecutionDisabledOperation {
    colors: Colors,
    alignment: Alignment,
    started: RefCell<bool>,
}

impl SnippetExecutionDisabledOperation {
    pub(crate) fn new(colors: Colors, alignment: Alignment) -> Self {
        Self { colors, alignment, started: Default::default() }
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
                line: vec![Text::new("snippet execution is disabled", TextStyle::default().colors(self.colors))].into(),
                alignment: self.alignment.clone(),
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
