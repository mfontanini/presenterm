use crate::{
    markdown::text::WeightedLine,
    render::{
        draw::{RenderError, RenderResult},
        layout::{Layout, Positioning},
        properties::WindowSize,
    },
    style::TextStyle,
    theme::{Alignment, Colors},
};
use crossterm::{cursor, style, QueueableCommand};
use std::io;

const MINIMUM_LINE_LENGTH: u16 = 10;

/// Draws text on the screen.
///
/// This deals with splitting words and doing word wrapping based on the given positioning.
pub struct TextDrawer<'a> {
    line: &'a WeightedLine,
    positioning: Positioning,
    default_colors: &'a Colors,
}

impl<'a> TextDrawer<'a> {
    pub fn new(
        alignment: &'a Alignment,
        line: &'a WeightedLine,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Result<Self, RenderError> {
        let text_length = line.width() as u16;
        let positioning = Layout(alignment).compute(dimensions, text_length);
        // If our line doesn't fit and it's just too small then abort
        if text_length > positioning.max_line_length && positioning.max_line_length <= MINIMUM_LINE_LENGTH {
            Err(RenderError::TerminalTooSmall)
        } else {
            Ok(Self { line, positioning, default_colors })
        }
    }

    /// Draw text on the given handle.
    ///
    /// This performs word splitting and word wrapping.
    pub fn draw<W: io::Write>(self, handle: &mut W) -> RenderResult {
        let Positioning { max_line_length, start_column } = self.positioning;
        handle.queue(cursor::MoveToColumn(start_column))?;

        for (line_index, line) in self.line.split(max_line_length as usize).enumerate() {
            handle.queue(cursor::MoveToColumn(start_column))?;
            if line_index > 0 {
                handle.queue(cursor::MoveDown(1))?;
            }
            for chunk in line {
                let (text, style) = chunk.into_parts();
                let text = style.apply(text);
                handle.queue(style::PrintStyledContent(text))?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if style != TextStyle::default() {
                    handle.queue(style::SetColors(style::Colors {
                        background: self.default_colors.background,
                        foreground: self.default_colors.foreground,
                    }))?;
                }
            }
        }
        Ok(())
    }
}
