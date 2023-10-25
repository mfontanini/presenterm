use super::{engine::RenderEngine, terminal::Terminal};
use crate::{
    markdown::{
        elements::StyledText,
        text::{WeightedLine, WeightedText},
    },
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
}

impl<W> TerminalDrawer<W>
where
    W: io::Write,
{
    /// Construct a drawer over a [std::io::Write].
    pub(crate) fn new(handle: W) -> io::Result<Self> {
        let terminal = Terminal::new(handle)?;
        Ok(Self { terminal })
    }

    /// Render a slide.
    pub(crate) fn render_slide(&mut self, presentation: &Presentation) -> RenderResult {
        let window_dimensions = WindowSize::current()?;
        let slide = presentation.current_slide();
        let engine = RenderEngine::new(&mut self.terminal, window_dimensions);
        engine.render(slide.iter_operations())?;
        self.terminal.flush()?;
        Ok(())
    }

    /// Render an error.
    pub(crate) fn render_error(&mut self, message: &str) -> RenderResult {
        let dimensions = WindowSize::current()?;
        let heading = vec![
            WeightedText::from(StyledText::new("Error loading presentation", TextStyle::default().bold())),
            WeightedText::from(StyledText::from(": ")),
        ];
        let error = vec![WeightedText::from(StyledText::from(message))];
        let alignment = Alignment::Center { minimum_size: 0, minimum_margin: Margin::Percent(8) };
        let operations = [
            RenderOperation::ClearScreen,
            RenderOperation::SetColors(Colors {
                foreground: Some(Color::new(255, 0, 0)),
                background: Some(Color::new(0, 0, 0)),
            }),
            RenderOperation::JumpToVerticalCenter,
            RenderOperation::RenderText { line: WeightedLine::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderText { line: WeightedLine::from(error), alignment: alignment.clone() },
        ];
        let engine = RenderEngine::new(&mut self.terminal, dimensions);
        engine.render(operations.iter())?;
        self.terminal.flush()?;
        Ok(())
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
