use super::terminal::{Terminal, TerminalWrite};
use crate::{
    markdown::{
        elements::Text,
        text::{WeightedLine, WeightedText},
    },
    render::{
        draw::{RenderError, RenderResult},
        layout::Positioning,
    },
    style::{Color, Colors},
};

const MINIMUM_LINE_LENGTH: u16 = 10;

/// Draws text on the screen.
///
/// This deals with splitting words and doing word wrapping based on the given positioning.
pub(crate) struct TextDrawer<'a> {
    prefix: &'a WeightedText,
    right_padding_length: u16,
    line: &'a WeightedLine,
    positioning: Positioning,
    prefix_length: u16,
    default_colors: &'a Colors,
    draw_block: bool,
    block_color: Option<Color>,
    repeat_prefix: bool,
}

impl<'a> TextDrawer<'a> {
    pub(crate) fn new(
        prefix: &'a WeightedText,
        right_padding_length: u16,
        line: &'a WeightedLine,
        positioning: Positioning,
        default_colors: &'a Colors,
    ) -> Result<Self, RenderError> {
        let text_length = (line.width() + prefix.width() + right_padding_length as usize) as u16;
        // If our line doesn't fit and it's just too small then abort
        if text_length > positioning.max_line_length && positioning.max_line_length <= MINIMUM_LINE_LENGTH {
            Err(RenderError::TerminalTooSmall)
        } else {
            let prefix_length = prefix.width() as u16;
            let positioning = Positioning {
                max_line_length: positioning
                    .max_line_length
                    .saturating_sub(prefix_length)
                    .saturating_sub(right_padding_length),
                start_column: positioning.start_column,
            };
            Ok(Self {
                prefix,
                right_padding_length,
                line,
                positioning,
                prefix_length,
                default_colors,
                draw_block: false,
                block_color: None,
                repeat_prefix: false,
            })
        }
    }

    pub(crate) fn with_surrounding_block(mut self, block_color: Option<Color>) -> Self {
        self.draw_block = true;
        self.block_color = block_color;
        self
    }

    pub(crate) fn repeat_prefix_on_wrap(mut self, value: bool) -> Self {
        self.repeat_prefix = value;
        self
    }

    /// Draw text on the given handle.
    ///
    /// This performs word splitting and word wrapping.
    pub(crate) fn draw<W>(self, terminal: &mut Terminal<W>) -> RenderResult
    where
        W: TerminalWrite,
    {
        let mut line_length: u16 = 0;

        // Print the prefix at the beginning of the line.
        let styled_prefix = {
            let Text { content, style } = self.prefix.text();
            style.apply(content)
        };
        terminal.move_to_column(self.positioning.start_column)?;
        terminal.print_styled_line(styled_prefix.clone())?;

        let start_column = self.positioning.start_column + self.prefix_length;
        for (line_index, line) in self.line.split(self.positioning.max_line_length as usize).enumerate() {
            if line_index > 0 {
                // Complete the current line's block to the right before moving down.
                self.print_block_background(line_length, terminal)?;
                terminal.move_down(1)?;
                line_length = 0;

                // Complete the new line in this block to the left where the prefix would be.
                if self.prefix_length > 0 {
                    terminal.move_to_column(self.positioning.start_column)?;
                    if self.repeat_prefix {
                        terminal.print_styled_line(styled_prefix.clone())?;
                    } else {
                        self.print_block_background(self.prefix_length, terminal)?;
                    }
                }
            }
            terminal.move_to_column(start_column)?;
            for chunk in line {
                line_length = line_length.saturating_add(chunk.width() as u16);

                let (text, style) = chunk.into_parts();
                let text = style.apply(text);
                terminal.print_styled_line(text)?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if style.has_modifiers() {
                    terminal.set_colors(*self.default_colors)?;
                }
            }
        }
        self.print_block_background(line_length, terminal)?;
        Ok(())
    }

    fn print_block_background<W>(&self, line_length: u16, terminal: &mut Terminal<W>) -> RenderResult
    where
        W: TerminalWrite,
    {
        if self.draw_block {
            let remaining =
                self.positioning.max_line_length.saturating_sub(line_length).saturating_add(self.right_padding_length);
            if remaining > 0 {
                if let Some(color) = self.block_color {
                    terminal.set_background_color(color)?;
                }
                let text = " ".repeat(remaining as usize);
                terminal.print_line(&text)?;
            }
        }
        Ok(())
    }
}
