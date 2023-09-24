use super::{
    draw::DrawResult,
    layout::{TextPositioning, WordWrapLayout},
};
use crate::{
    markdown::text::WeightedLine,
    style::TextStyle,
    theme::{Alignment, Colors},
};
use crossterm::{cursor, style, terminal::WindowSize, QueueableCommand};
use std::io;

pub(crate) struct TextDrawer<'a, W> {
    handle: &'a mut W,
    line: &'a WeightedLine,
    positioning: TextPositioning,
    default_colors: &'a Colors,
}

impl<'a, W> TextDrawer<'a, W>
where
    W: io::Write,
{
    pub(crate) fn new(
        alignment: &'a Alignment,
        handle: &'a mut W,
        line: &'a WeightedLine,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Self {
        let text_length = line.width() as u16;
        let positioning = WordWrapLayout(alignment).compute(dimensions, text_length);
        Self { handle, line, positioning, default_colors }
    }

    pub(crate) fn draw(self) -> DrawResult {
        let TextPositioning { line_length, start_column } = self.positioning;
        self.handle.queue(cursor::MoveToColumn(start_column))?;

        for (line_index, line) in self.line.split(line_length as usize).enumerate() {
            self.handle.queue(cursor::MoveToColumn(start_column))?;
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
