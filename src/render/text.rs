use crate::{
    markdown::text::WeightedLine,
    style::TextStyle,
    theme::{Alignment, Colors},
};
use crossterm::{cursor, style, terminal::WindowSize, QueueableCommand};
use std::io;

use super::draw::DrawResult;

pub(super) struct TextDrawer<'a, W> {
    handle: &'a mut W,
    line: &'a WeightedLine,
    start_column: u16,
    line_length: u16,
    default_colors: &'a Colors,
}

impl<'a, W> TextDrawer<'a, W>
where
    W: io::Write,
{
    pub(super) fn new(
        alignment: &'a Alignment,
        handle: &'a mut W,
        line: &'a WeightedLine,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Self {
        let text_length = line.width() as u16;
        let mut line_length = dimensions.columns;
        let mut start_column;
        match *alignment {
            Alignment::Left { margin } => {
                start_column = margin;
                line_length -= margin * 2;
            }
            Alignment::Center { minimum_margin, minimum_size } => {
                line_length = text_length.min(dimensions.columns - minimum_margin * 2).max(minimum_size);
                if line_length > dimensions.columns {
                    start_column = minimum_margin;
                } else {
                    start_column = (dimensions.columns - line_length) / 2;
                    start_column = start_column.max(minimum_margin);
                }
            }
        };
        Self { handle, line, start_column, line_length, default_colors }
    }

    pub(super) fn draw(self) -> DrawResult {
        self.handle.queue(cursor::MoveToColumn(self.start_column))?;

        for (line_index, line) in self.line.split(self.line_length as usize).enumerate() {
            self.handle.queue(cursor::MoveToColumn(self.start_column))?;
            if line_index > 0 {
                self.handle.queue(cursor::MoveDown(1))?;
            }
            for chunk in line {
                let (text, style) = chunk.into_parts();
                let text = style.apply(text);
                self.handle.queue(style::PrintStyledContent(text))?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if style != TextStyle::default() {
                    self.handle.queue(style::SetColors(style::Colors {
                        background: self.default_colors.background,
                        foreground: self.default_colors.foreground,
                    }))?;
                }
            }
        }
        Ok(())
    }
}
