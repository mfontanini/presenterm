use super::terminal::{Terminal, TerminalWrite};
use crate::{
    markdown::text::WeightedTextBlock,
    render::{
        draw::{RenderError, RenderResult},
        layout::{Layout, Positioning},
        properties::WindowSize,
    },
    style::{Colors, TextStyle},
};

const MINIMUM_LINE_LENGTH: u16 = 10;

/// Draws text on the screen.
///
/// This deals with splitting words and doing word wrapping based on the given positioning.
pub(crate) struct TextDrawer<'a> {
    line: &'a WeightedTextBlock,
    positioning: Positioning,
    default_colors: &'a Colors,
    extend_block: bool,
}

impl<'a> TextDrawer<'a> {
    pub(crate) fn new(
        layout: &Layout,
        line: &'a WeightedTextBlock,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Result<Self, RenderError> {
        let text_length = line.width() as u16;
        let positioning = layout.compute(dimensions, text_length);
        // If our line doesn't fit and it's just too small then abort
        if text_length > positioning.max_line_length && positioning.max_line_length <= MINIMUM_LINE_LENGTH {
            Err(RenderError::TerminalTooSmall)
        } else {
            Ok(Self { line, positioning, default_colors, extend_block: false })
        }
    }

    pub(crate) fn new_block(
        line: &'a WeightedTextBlock,
        positioning: Positioning,
        default_colors: &'a Colors,
    ) -> Result<Self, RenderError> {
        let text_length = line.width() as u16;
        // If our line doesn't fit and it's just too small then abort
        if text_length > positioning.max_line_length && positioning.max_line_length <= MINIMUM_LINE_LENGTH {
            Err(RenderError::TerminalTooSmall)
        } else {
            Ok(Self { line, positioning, default_colors, extend_block: true })
        }
    }

    /// Draw text on the given handle.
    ///
    /// This performs word splitting and word wrapping.
    pub(crate) fn draw<W>(self, terminal: &mut Terminal<W>) -> RenderResult
    where
        W: TerminalWrite,
    {
        let Positioning { max_line_length, start_column } = self.positioning;

        let mut line_length: u16 = 0;
        for (line_index, line) in self.line.split(max_line_length as usize).enumerate() {
            if line_index > 0 {
                self.print_block_background(line_length, max_line_length, terminal)?;
                terminal.move_down(1)?;
                line_length = 0;
            }
            terminal.move_to_column(start_column)?;
            for chunk in line {
                line_length = line_length.saturating_add(chunk.width() as u16);

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
        self.print_block_background(line_length, max_line_length, terminal)?;
        Ok(())
    }

    fn print_block_background<W>(
        &self,
        line_length: u16,
        max_line_length: u16,
        terminal: &mut Terminal<W>,
    ) -> RenderResult
    where
        W: TerminalWrite,
    {
        if self.extend_block {
            let remaining = max_line_length.saturating_sub(line_length);
            if remaining > 0 {
                let text = " ".repeat(remaining as usize);
                terminal.print_line(&text)?;
            }
        }
        Ok(())
    }
}
