use super::{engine::RenderEngine, media::GraphicsMode, terminal::Terminal};
use crate::{
    markdown::{elements::Text, text::WeightedTextBlock},
    presentation::{Presentation, RenderOperation},
    render::properties::WindowSize,
    style::{Color, Colors, TextStyle},
    theme::{Alignment, Margin},
};
use std::io;

/// The result of a render operation.
pub(crate) type RenderResult = Result<(), RenderError>;

/// Allows drawing elements in the terminal.
pub(crate) struct TerminalDrawer<W: io::Write> {
    terminal: Terminal<W>,
    graphics_mode: GraphicsMode,
    font_size_fallback: Option<u8>,
}

impl<W> TerminalDrawer<W>
where
    W: io::Write,
{
    /// Construct a drawer over a [std::io::Write].
    pub(crate) fn new(handle: W, graphics_mode: GraphicsMode, font_size_fallback: Option<u8>) -> io::Result<Self> {
        let terminal = Terminal::new(handle)?;
        Ok(Self { terminal, graphics_mode, font_size_fallback })
    }

    /// Render a slide.
    pub(crate) fn render_slide(&mut self, presentation: &Presentation) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
        let slide = presentation.current_slide();
        let engine = self.create_engine(dimensions);
        engine.render(slide.iter_operations())?;
        Ok(())
    }

    /// Render an error.
    pub(crate) fn render_error(&mut self, message: &str) -> RenderResult {
        let dimensions = WindowSize::current(self.font_size_fallback)?;
        let heading = vec![Text::new("Error loading presentation", TextStyle::default().bold()), Text::from(": ")];

        let alignment = Alignment::Center { minimum_size: 0, minimum_margin: Margin::Percent(8) };
        let mut operations = vec![
            RenderOperation::ClearScreen,
            RenderOperation::SetColors(Colors {
                foreground: Some(Color::new(255, 0, 0)),
                background: Some(Color::new(0, 0, 0)),
            }),
            RenderOperation::JumpToVerticalCenter,
            RenderOperation::RenderText { line: WeightedTextBlock::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
        ];
        for line in message.lines() {
            let error = vec![Text::from(line)];
            let op = RenderOperation::RenderText { line: WeightedTextBlock::from(error), alignment: alignment.clone() };
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
        RenderEngine::new(&mut self.terminal, dimensions, self.graphics_mode.clone())
    }
}

/// A rendering error.
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("unsupported structure: {0}")]
    UnsupportedStructure(&'static str),

    #[error("screen is too small")]
    TerminalTooSmall,

    #[error("tried to move to non existent layout location")]
    InvalidLayoutEnter,

    #[error("tried to pop default screen")]
    PopDefaultScreen,

    #[error(transparent)]
    Other(Box<dyn std::error::Error>),
}
