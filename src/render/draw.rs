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
    terminal::{disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use std::io;

pub type RenderResult = Result<(), RenderError>;

pub struct Drawer<W: io::Write> {
    handle: W,
}

impl<W> Drawer<W>
where
    W: io::Write,
{
    pub fn new(mut handle: W) -> io::Result<Self> {
        enable_raw_mode()?;
        handle.queue(cursor::Hide)?;
        Ok(Self { handle })
    }

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

    pub fn render_error(&mut self, message: &str) -> RenderResult {
        let dimensions = WindowSize::current()?;
        let heading = vec![
            WeightedText::from(StyledText::styled("Error loading presentation", TextStyle::default().bold())),
            WeightedText::from(StyledText::plain(": ")),
        ];
        let error = vec![WeightedText::from(StyledText::plain(message))];
        let alignment = Alignment::Center { minimum_size: 0, minimum_margin: 5 };
        let operations = vec![
            RenderOperation::ClearScreen,
            RenderOperation::SetColors(Colors { foreground: Some(Color::Red), background: Some(Color::Black) }),
            RenderOperation::JumpToVerticalCenter,
            RenderOperation::RenderTextLine { texts: WeightedLine::from(heading), alignment: alignment.clone() },
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderTextLine { texts: WeightedLine::from(error), alignment: alignment.clone() },
        ];
        let mut operator = RenderOperator::new(&mut self.handle, dimensions.clone(), dimensions, Default::default());
        for operation in operations {
            operator.render(&operation)?;
        }
        self.handle.flush()?;
        Ok(())
    }
}

impl<W> Drop for Drawer<W>
where
    W: io::Write,
{
    fn drop(&mut self) {
        let _ = self.handle.queue(cursor::Show);
        let _ = disable_raw_mode();
    }
}

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
