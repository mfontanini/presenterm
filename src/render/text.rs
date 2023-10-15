use super::terminal::Terminal;
use crate::{
    markdown::text::WeightedLine,
    render::{
        draw::{RenderError, RenderResult},
        layout::{Layout, Positioning},
        properties::WindowSize,
    },
    style::{Colors, TextStyle},
};
use std::io;

const MINIMUM_LINE_LENGTH: u16 = 10;

/// Draws text on the screen.
///
/// This deals with splitting words and doing word wrapping based on the given positioning.
pub(crate) struct TextDrawer<'a> {
    line: &'a WeightedLine,
    positioning: Positioning,
    default_colors: &'a Colors,
}

impl<'a> TextDrawer<'a> {
    pub(crate) fn new(
        layout: &Layout,
        line: &'a WeightedLine,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Result<Self, RenderError> {
        let text_length = line.width() as u16;
        let positioning = layout.compute(dimensions, text_length);
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
    pub(crate) fn draw<W>(self, terminal: &mut Terminal<W>) -> RenderResult
    where
        W: io::Write,
    {
        let Positioning { max_line_length, start_column } = self.positioning;

        for (line_index, line) in self.line.split(max_line_length as usize).enumerate() {
            terminal.move_to_column(start_column)?;
            if line_index > 0 {
                terminal.move_down(1)?;
            }
            for chunk in line {
                let (text, style) = chunk.into_parts();
                let text = style.apply(text);
                terminal.print_styled_line(text)?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if style != TextStyle::default() {
                    terminal.set_colors(self.default_colors.clone())?;
                }
            }
        }
        Ok(())
    }
}
