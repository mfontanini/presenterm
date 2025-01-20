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
        text_style::{Color, Colors, TextStyle},
    },
    render::{operation::RenderOperation, properties::WindowSize},
    terminal::{
        Terminal, TerminalWrite,
        image::printer::{ImagePrinter, PrintImageError},
    },
    theme::{Alignment, Margin},
};
use engine::{RenderEngine, RenderEngineOptions};
use std::{io, sync::Arc};

/// The result of a render operation.
pub(crate) type RenderResult = Result<(), RenderError>;

pub(crate) struct TerminalDrawerOptions {
    /// The font size to fall back to if we can't find the window size in pixels.
    pub(crate) font_size_fallback: u8,

    /// The max width in columns that the presentation should be capped to.
    pub(crate) max_columns: u16,
}

impl Default for TerminalDrawerOptions {
    fn default() -> Self {
        Self { font_size_fallback: 1, max_columns: u16::MAX }
    }
}

/// Allows drawing on the terminal.
pub(crate) struct TerminalDrawer<W: TerminalWrite> {
    pub(crate) terminal: Terminal<W>,
    options: TerminalDrawerOptions,
}

impl<W> TerminalDrawer<W>
where
    W: TerminalWrite,
{
    pub(crate) fn new(handle: W, image_printer: Arc<ImagePrinter>, options: TerminalDrawerOptions) -> io::Result<Self> {
        let terminal = Terminal::new(handle, image_printer)?;
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
        let dimensions = WindowSize::current(self.options.font_size_fallback)?;
        let heading_text = match source {
            ErrorSource::Presentation => "Error loading presentation".to_string(),
            ErrorSource::Slide(slide) => {
                format!("Error in slide {slide}")
            }
        };
        let heading = vec![Text::new(heading_text, TextStyle::default().bold()), Text::from(": ")];
        let total_lines = message.lines().count();
        let starting_row = (dimensions.rows / 2).saturating_sub(total_lines as u16 / 2 + 3);
        let alignment = Alignment::Left { margin: Margin::Percent(25) };

        let mut operations = vec![
            RenderOperation::SetColors(Colors {
                foreground: Some(Color::new(255, 0, 0)),
                background: Some(Color::new(0, 0, 0)),
            }),
            RenderOperation::ClearScreen,
            RenderOperation::JumpToRow { index: starting_row },
            RenderOperation::RenderText { line: WeightedLine::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
        ];
        for line in message.lines() {
            let error = vec![Text::from(line)];
            let op = RenderOperation::RenderText { line: WeightedLine::from(error), alignment: alignment.clone() };
            operations.extend([op, RenderOperation::RenderLineBreak]);
        }
        let engine = self.create_engine(dimensions);
        engine.render(operations.iter())?;
        Ok(())
    }

    fn create_engine(&mut self, dimensions: WindowSize) -> RenderEngine<W> {
        let options = RenderEngineOptions { max_columns: self.options.max_columns, ..Default::default() };
        RenderEngine::new(&mut self.terminal, dimensions, options)
    }
}

/// A rendering error.
#[derive(thiserror::Error, Debug)]
pub(crate) enum RenderError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

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
}

pub(crate) enum ErrorSource {
    Presentation,
    Slide(usize),
}
