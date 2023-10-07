use super::operator::RenderOperator;
use crate::{
    markdown::{
        elements::StyledText,
        text::{WeightedLine, WeightedText},
    },
    presentation::{Presentation, RenderOperation},
    render::properties::WindowSize,
    style::TextStyle,
    theme::{Alignment, Colors},
};
use crossterm::{
    cursor,
    style::Color,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use std::io;

/// The result of a render operation.
pub type RenderResult = Result<(), RenderError>;

/// Allows drawing elements in the terminal.
pub struct TerminalDrawer<W: io::Write> {
    handle: W,
}

impl<W> TerminalDrawer<W>
where
    W: io::Write,
{
    /// Construct a drawer over a [std::io::Write].
    pub fn new(mut handle: W) -> io::Result<Self> {
        enable_raw_mode()?;
        handle.queue(cursor::Hide)?;
        handle.queue(terminal::EnterAlternateScreen)?;
        Ok(Self { handle })
    }

    /// Render a slide.
    pub fn render_slide(&mut self, presentation: &Presentation) -> RenderResult {
        let window_dimensions = WindowSize::current()?;
        let slide_dimensions = window_dimensions.shrink_rows(3);
        let slide = presentation.current_slide();
        let mut operator =
            RenderOperator::new(&mut self.handle, slide_dimensions, window_dimensions, Default::default());
        for element in &slide.render_operations {
            operator.render(element)?;
        }
        self.handle.flush()?;
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
        let alignment = Alignment::Center { minimum_size: 0, minimum_margin: 5 };
        let operations = vec![
            RenderOperation::ClearScreen,
            RenderOperation::SetColors(Colors { foreground: Some(Color::Red), background: Some(Color::Black) }),
            RenderOperation::JumpToVerticalCenter,
            RenderOperation::RenderTextLine { line: WeightedLine::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderTextLine { line: WeightedLine::from(error), alignment: alignment.clone() },
        ];
        let mut operator = RenderOperator::new(&mut self.handle, dimensions.clone(), dimensions, Default::default());
        for operation in operations {
            operator.render(&operation)?;
        }
        self.handle.flush()?;
        Ok(())
    }
}

impl<W> Drop for TerminalDrawer<W>
where
    W: io::Write,
{
    fn drop(&mut self) {
        let _ = self.handle.queue(terminal::LeaveAlternateScreen);
        let _ = self.handle.queue(cursor::Show);
        let _ = disable_raw_mode();
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

    #[error(transparent)]
    Other(Box<dyn std::error::Error>),
}
