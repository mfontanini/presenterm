use super::engine::RenderEngine;
use crate::{
    markdown::{
        elements::Text,
        text::WeightedLine,
        text_style::{Color, Colors, TextStyle},
    },
    presentation::Presentation,
    render::{operation::RenderOperation, properties::WindowSize},
    terminal::{
        Terminal, TerminalWrite,
        image::printer::{ImagePrinter, PrintImageError},
    },
    theme::{Alignment, Margin},
};
use std::{io, sync::Arc};

/// The result of a render operation.
pub(crate) type RenderResult = Result<(), RenderError>;

/// Allows drawing elements in the terminal.
pub(crate) struct TerminalDrawer<W: TerminalWrite> {
    pub(crate) terminal: Terminal<W>,
    font_size_fallback: u8,
}

impl<W> TerminalDrawer<W>
where
    W: TerminalWrite,
{
    /// Construct a drawer over a [std::io::Write].
    pub(crate) fn new(handle: W, image_printer: Arc<ImagePrinter>, font_size_fallback: u8) -> io::Result<Self> {
        let terminal = Terminal::new(handle, image_printer)?;
        Ok(Self { terminal, font_size_fallback })
    }

    /// Render a slide.
    pub(crate) fn render_slide(&mut self, presentation: &Presentation) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
        let slide = presentation.current_slide();
        let engine = self.create_engine(dimensions);
        engine.render(slide.iter_visible_operations())?;
        Ok(())
    }

    /// Render an error.
    pub(crate) fn render_error(&mut self, message: &str, source: &ErrorSource) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
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

    pub(crate) fn render_slide_index(&mut self, presentation: &Presentation) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
        let engine = self.create_engine(dimensions);
        engine.render(presentation.iter_slide_index_operations())?;
        Ok(())
    }

    pub(crate) fn render_key_bindings(&mut self, presentation: &Presentation) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
        let engine = self.create_engine(dimensions);
        engine.render(presentation.iter_bindings_operations())?;
        Ok(())
    }

    fn create_engine(&mut self, dimensions: WindowSize) -> RenderEngine<W> {
        let options = Default::default();
        RenderEngine::new(&mut self.terminal, dimensions, options)
    }
}

/// A rendering error.
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
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
