pub(crate) mod ascii_scaler;
pub(crate) mod engine;
pub(crate) mod layout;
pub(crate) mod operation;
pub(crate) mod properties;
pub(crate) mod text;
pub(crate) mod validate;

use crate::{
    markdown::{
        elements::Text,
        text::WeightedLine,
        text_style::{Color, Colors, PaletteColorError, TextStyle},
    },
    render::{operation::RenderOperation, properties::WindowSize},
    terminal::{
        Terminal,
        image::printer::{ImagePrinter, PrintImageError},
        printer::TerminalError,
    },
    theme::{Alignment, Margin},
};
use engine::{MaxSize, RenderEngine, RenderEngineOptions};
use operation::AsRenderOperations;
use std::{
    io::{self, Stdout},
    iter,
    rc::Rc,
    sync::Arc,
};

/// The result of a render operation.
pub(crate) type RenderResult = Result<(), RenderError>;

pub(crate) struct TerminalDrawerOptions {
    pub(crate) font_size_fallback: u8,
    pub(crate) max_size: MaxSize,
}

impl Default for TerminalDrawerOptions {
    fn default() -> Self {
        Self { font_size_fallback: 1, max_size: Default::default() }
    }
}

/// Allows drawing on the terminal.
pub(crate) struct TerminalDrawer {
    pub(crate) terminal: Terminal<Stdout>,
    options: TerminalDrawerOptions,
}

impl TerminalDrawer {
    pub(crate) fn new(image_printer: Arc<ImagePrinter>, options: TerminalDrawerOptions) -> io::Result<Self> {
        let terminal = Terminal::new(io::stdout(), image_printer)?;
        Ok(Self { terminal, options })
    }

    pub(crate) fn render_operations<'a>(
        &mut self,
        operations: impl Iterator<Item = &'a RenderOperation>,
    ) -> RenderResult {
        let dimensions = WindowSize::current(self.options.font_size_fallback)?;
        let engine = self.create_engine(dimensions);
        engine.render(operations)?;
        Ok(())
    }

    pub(crate) fn render_error(&mut self, message: &str, source: &ErrorSource) -> RenderResult {
        let operation = RenderErrorOperation { message: message.into(), source: source.clone() };
        let operation = RenderOperation::RenderDynamic(Rc::new(operation));
        let dimensions = WindowSize::current(self.options.font_size_fallback)?;
        let engine = self.create_engine(dimensions);
        engine.render(iter::once(&operation))?;
        Ok(())
    }

    pub(crate) fn render_engine_options(&self) -> RenderEngineOptions {
        RenderEngineOptions { max_size: self.options.max_size.clone(), ..Default::default() }
    }

    fn create_engine(&mut self, dimensions: WindowSize) -> RenderEngine<Terminal<Stdout>> {
        let options = self.render_engine_options();
        RenderEngine::new(&mut self.terminal, dimensions, options)
    }
}

/// A rendering error.
#[derive(thiserror::Error, Debug)]
pub(crate) enum RenderError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("terminal: {0}")]
    Terminal(#[from] TerminalError),

    #[error("screen is too small")]
    TerminalTooSmall,

    #[error("tried to move to non existent layout location")]
    InvalidLayoutEnter,

    #[error("tried to pop default screen")]
    PopDefaultScreen,

    #[error("printing image: {0}")]
    PrintImage(#[from] PrintImageError),

    #[error("horizontal overflow")]
    HorizontalOverflow,

    #[error("vertical overflow")]
    VerticalOverflow,

    #[error(transparent)]
    PaletteColor(#[from] PaletteColorError),
}

#[derive(Clone, Debug)]
pub(crate) enum ErrorSource {
    Presentation,
    Slide(usize),
}

#[derive(Debug)]
struct RenderErrorOperation {
    message: String,
    source: ErrorSource,
}

impl AsRenderOperations for RenderErrorOperation {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let heading_text = match self.source {
            ErrorSource::Presentation => "Error loading presentation".to_string(),
            ErrorSource::Slide(slide) => {
                format!("Error in slide {slide}")
            }
        };
        let heading = vec![Text::new(heading_text, TextStyle::default().bold()), Text::from(": ")];
        let total_lines = self.message.lines().count();
        let starting_row = (dimensions.rows / 2).saturating_sub(total_lines as u16 / 2 + 3);
        let alignment = Alignment::Left { margin: Margin::Percent(25) };

        let mut operations = vec![
            RenderOperation::SetColors(Colors {
                foreground: Some(Color::new(255, 0, 0)),
                background: Some(Color::new(0, 0, 0)),
            }),
            RenderOperation::ClearScreen,
            RenderOperation::JumpToRow { index: starting_row },
            RenderOperation::RenderText { line: WeightedLine::from(heading), alignment },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
        ];
        for line in self.message.lines() {
            let error = vec![Text::from(line)];
            let op = RenderOperation::RenderText { line: WeightedLine::from(error), alignment };
            operations.extend([op, RenderOperation::RenderLineBreak]);
        }
        operations
    }
}
