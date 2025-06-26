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
        ansi::AnsiParser,
        image::printer::{ImagePrinter, PrintImageError},
        printer::TerminalError,
    },
    theme::Margin,
};
use engine::{MaxSize, RenderEngine, RenderEngineOptions};
use operation::{AsRenderOperations, MarginProperties};
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
        let (lines, _) = AnsiParser::new(Default::default()).parse_lines(message.lines());
        let lines = lines.into_iter().map(Into::into).collect();
        let operation = RenderErrorOperation { lines, source: source.clone() };
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
    lines: Vec<WeightedLine>,
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
        let heading = vec![Text::new(heading_text, TextStyle::default().bold().fg_color(Color::Red)), Text::from(": ")];
        let content_width: u16 =
            self.lines.iter().map(|l| l.width()).max().unwrap_or_default().try_into().unwrap_or(u16::MAX);
        let minimum_margin = (dimensions.columns as f32 * 0.1) as u16;
        let margin = dimensions.columns.saturating_sub(content_width).max(minimum_margin) / 2;

        let total_lines = self.lines.len();
        let starting_row = (dimensions.rows / 2).saturating_sub(total_lines as u16 / 2 + 3);

        let mut operations = vec![
            RenderOperation::SetColors(Colors {
                background: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
                foreground: Some(Color::White),
            }),
            RenderOperation::ClearScreen,
            RenderOperation::ApplyMargin(MarginProperties {
                horizontal: Margin::Fixed(margin),
                top: starting_row,
                bottom: 0,
            }),
            RenderOperation::RenderText { line: WeightedLine::from(heading), alignment: Default::default() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
        ];
        for line in self.lines.iter().cloned() {
            let op = RenderOperation::RenderText { line, alignment: Default::default() };
            operations.extend([op, RenderOperation::RenderLineBreak]);
        }
        operations
    }
}
