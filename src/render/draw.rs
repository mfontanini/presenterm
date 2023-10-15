use super::{operator::RenderOperator, terminal::Terminal};
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
pub type RenderResult = Result<(), RenderError>;

/// Allows drawing elements in the terminal.
pub struct TerminalDrawer<W: io::Write> {
    terminal: Terminal<W>,
}

impl<W> TerminalDrawer<W>
where
    W: io::Write,
{
    /// Construct a drawer over a [std::io::Write].
    pub fn new(handle: W) -> io::Result<Self> {
        let terminal = Terminal::new(handle)?;
        Ok(Self { terminal })
    }

    /// Render a slide.
    pub fn render_slide(&mut self, presentation: &Presentation) -> RenderResult {
        let window_dimensions = WindowSize::current()?;
        let slide_dimensions = window_dimensions.shrink_rows(3);
        let slide = presentation.current_slide();
        let operator = RenderOperator::new(&mut self.terminal, slide_dimensions, window_dimensions);
        operator.render(&slide.render_operations)?;
        self.terminal.flush()?;
        Ok(())
    }

    /// Render an error.
    pub fn render_error(&mut self, message: &str) -> RenderResult {
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
            RenderOperation::RenderTextLine { line: WeightedLine::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderTextLine { line: WeightedLine::from(error), alignment: alignment.clone() },
        ];
        let operator = RenderOperator::new(&mut self.terminal, dimensions.clone(), dimensions);
        operator.render(&operations)?;
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

    #[error(transparent)]
    Other(Box<dyn std::error::Error>),
}
