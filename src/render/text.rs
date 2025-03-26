use crate::{
    markdown::{
        elements::Text,
        text::{WeightedLine, WeightedText},
        text_style::{Color, Colors, TextStyle},
    },
    render::{RenderError, RenderResult, layout::Positioning},
    terminal::printer::{TerminalCommand, TerminalIo, TextProperties},
};

/// Draws text on the screen.
///
/// This deals with splitting words and doing word wrapping based on the given positioning.
pub(crate) struct TextDrawer<'a> {
    prefix: &'a WeightedText,
    right_padding_length: u16,
    line: &'a WeightedLine,
    positioning: Positioning,
    prefix_width: u16,
    default_colors: &'a Colors,
    draw_block: bool,
    block_color: Option<Color>,
    repeat_prefix: bool,
    properties: TextProperties,
}

impl<'a> TextDrawer<'a> {
    pub(crate) fn new(
        prefix: &'a WeightedText,
        right_padding_length: u16,
        line: &'a WeightedLine,
        positioning: Positioning,
        default_colors: &'a Colors,
        minimum_line_length: u16,
    ) -> Result<Self, RenderError> {
        let text_length = (line.width() + prefix.width() + right_padding_length as usize) as u16;
        // If our line doesn't fit and it's just too small then abort
        if text_length > positioning.max_line_length && positioning.max_line_length <= minimum_line_length {
            Err(RenderError::TerminalTooSmall)
        } else {
            let prefix_width = prefix.width() as u16;
            let positioning = Positioning {
                max_line_length: positioning
                    .max_line_length
                    .saturating_sub(prefix_width)
                    .saturating_sub(right_padding_length),
                start_column: positioning.start_column,
            };
            Ok(Self {
                prefix,
                right_padding_length,
                line,
                positioning,
                prefix_width,
                default_colors,
                draw_block: false,
                block_color: None,
                repeat_prefix: false,
                properties: TextProperties { height: line.font_size() },
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
    pub(crate) fn draw<T>(self, terminal: &mut T) -> RenderResult
    where
        T: TerminalIo,
    {
        let mut line_length: u16 = 0;
        terminal.execute(&TerminalCommand::MoveToColumn(self.positioning.start_column))?;

        // Print the prefix at the beginning of the line.
        if self.prefix_width > 0 {
            let Text { content, style } = self.prefix.text();
            terminal.execute(&TerminalCommand::PrintText { content, style: *style, properties: self.properties })?;
        }
        for (line_index, line) in self.line.split(self.positioning.max_line_length as usize).enumerate() {
            if line_index > 0 {
                // Complete the current line's block to the right before moving down.
                self.print_block_background(line_length, terminal)?;
                terminal.execute(&TerminalCommand::MoveDown(self.properties.height as u16))?;
                terminal.execute(&TerminalCommand::MoveToColumn(self.positioning.start_column))?;
                line_length = 0;

                // Complete the new line in this block to the left where the prefix would be.
                if self.prefix_width > 0 {
                    if self.repeat_prefix {
                        let Text { content, style } = self.prefix.text();
                        terminal.execute(&TerminalCommand::PrintText {
                            content,
                            style: *style,
                            properties: self.properties,
                        })?;
                    } else {
                        if let Some(color) = self.block_color {
                            terminal.execute(&TerminalCommand::SetBackgroundColor(color))?;
                        }
                        let text = " ".repeat(self.prefix_width as usize / self.properties.height as usize);
                        let style = TextStyle::default().size(self.properties.height);
                        terminal.execute(&TerminalCommand::PrintText {
                            content: &text,
                            style,
                            properties: self.properties,
                        })?;
                    }
                }
            }
            for chunk in line {
                line_length = line_length.saturating_add(chunk.width() as u16);

                let (text, style) = chunk.into_parts();
                terminal.execute(&TerminalCommand::PrintText { content: text, style, properties: self.properties })?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if style != Default::default() {
                    terminal.execute(&TerminalCommand::SetColors(*self.default_colors))?;
                }
            }
        }
        self.print_block_background(line_length, terminal)?;
        Ok(())
    }

    fn print_block_background<T>(&self, line_length: u16, terminal: &mut T) -> RenderResult
    where
        T: TerminalIo,
    {
        if self.draw_block {
            let remaining =
                self.positioning.max_line_length.saturating_sub(line_length).saturating_add(self.right_padding_length);
            if remaining > 0 {
                if let Some(color) = self.block_color {
                    terminal.execute(&TerminalCommand::SetBackgroundColor(color))?;
                }
                let text = " ".repeat(remaining as usize / self.properties.height as usize);
                let style = TextStyle::default().size(self.properties.height);
                terminal.execute(&TerminalCommand::PrintText { content: &text, style, properties: self.properties })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::printer::TerminalError;
    use std::io;
    use unicode_width::UnicodeWidthStr;

    #[derive(Debug, PartialEq)]
    enum Instruction {
        MoveDown(u16),
        MoveToColumn(u16),
        PrintText { content: String, font_size: u8 },
    }

    #[derive(Default)]
    struct TerminalBuf {
        instructions: Vec<Instruction>,
        cursor_row: u16,
    }

    impl TerminalBuf {
        fn push(&mut self, instruction: Instruction) -> io::Result<()> {
            self.instructions.push(instruction);
            Ok(())
        }

        fn move_to_column(&mut self, column: u16) -> std::io::Result<()> {
            self.push(Instruction::MoveToColumn(column))
        }

        fn move_down(&mut self, amount: u16) -> std::io::Result<()> {
            self.push(Instruction::MoveDown(amount))
        }

        fn print_text(&mut self, content: &str, style: &TextStyle, _properties: &TextProperties) -> io::Result<()> {
            let content = content.to_string();
            if content.is_empty() {
                return Ok(());
            }
            self.cursor_row = content.width() as u16;
            self.push(Instruction::PrintText { content, font_size: style.size })?;
            Ok(())
        }

        fn clear_screen(&mut self) -> std::io::Result<()> {
            unimplemented!()
        }

        fn set_colors(&mut self, _colors: Colors) -> std::io::Result<()> {
            Ok(())
        }

        fn set_background_color(&mut self, _color: Color) -> std::io::Result<()> {
            Ok(())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl TerminalIo for TerminalBuf {
        fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
            use TerminalCommand::*;
            match command {
                BeginUpdate | EndUpdate | MoveToRow(_) | MoveToNextLine | MoveTo { .. } | PrintImage { .. } => {
                    unimplemented!()
                }
                MoveToColumn(column) => self.move_to_column(*column)?,
                MoveDown(amount) => self.move_down(*amount)?,
                PrintText { content, style, properties } => self.print_text(content, style, properties)?,
                ClearScreen => self.clear_screen()?,
                SetColors(colors) => self.set_colors(*colors)?,
                SetBackgroundColor(color) => self.set_background_color(*color)?,
                Flush => self.flush()?,
            };
            Ok(())
        }

        fn cursor_row(&self) -> u16 {
            self.cursor_row
        }
    }

    struct TestDrawer {
        prefix: WeightedText,
        positioning: Positioning,
        right_padding_length: u16,
        repeat_prefix_on_wrap: bool,
    }

    impl TestDrawer {
        fn prefix<T: Into<WeightedText>>(mut self, prefix: T) -> Self {
            self.prefix = prefix.into();
            self
        }

        fn start_column(mut self, column: u16) -> Self {
            self.positioning.start_column = column;
            self
        }

        fn max_line_length(mut self, length: u16) -> Self {
            self.positioning.max_line_length = length;
            self
        }

        fn repeat_prefix_on_wrap(mut self) -> Self {
            self.repeat_prefix_on_wrap = true;
            self
        }

        fn draw<L: Into<WeightedLine>>(self, line: L) -> Vec<Instruction> {
            let line = line.into();
            let colors = Default::default();
            let drawer = TextDrawer::new(&self.prefix, self.right_padding_length, &line, self.positioning, &colors, 0)
                .expect("failed to create drawer")
                .repeat_prefix_on_wrap(self.repeat_prefix_on_wrap);
            let mut buf = TerminalBuf::default();
            drawer.draw(&mut buf).expect("drawing failed");
            buf.instructions
        }
    }

    impl Default for TestDrawer {
        fn default() -> Self {
            Self {
                prefix: WeightedText::from(""),
                positioning: Positioning { max_line_length: 100, start_column: 0 },
                right_padding_length: 0,
                repeat_prefix_on_wrap: false,
            }
        }
    }

    #[test]
    fn prefix_on_long_line() {
        let instructions = TestDrawer::default().prefix("P").max_line_length(3).start_column(1).draw("AAAA");
        let expected = &[
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: "P".into(), font_size: 1 },
            Instruction::PrintText { content: "AA".into(), font_size: 1 },
            Instruction::MoveDown(1),
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: " ".into(), font_size: 1 },
            Instruction::PrintText { content: "AA".into(), font_size: 1 },
        ];
        assert_eq!(instructions, expected);
    }

    #[test]
    fn prefix_on_long_line_with_font_size() {
        let text = WeightedLine::from(vec![Text::new("AAAA", TextStyle::default().size(2))]);
        let prefix = WeightedText::from(Text::new("P", TextStyle::default().size(2)));
        let instructions = TestDrawer::default().prefix(prefix).max_line_length(6).start_column(1).draw(text);
        let expected = &[
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: "P".into(), font_size: 2 },
            Instruction::PrintText { content: "AA".into(), font_size: 2 },
            Instruction::MoveDown(2),
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: " ".into(), font_size: 2 },
            Instruction::PrintText { content: "AA".into(), font_size: 2 },
        ];
        assert_eq!(instructions, expected);
    }

    #[test]
    fn prefix_on_long_line_with_font_size_and_repeat_prefix() {
        let text = WeightedLine::from(vec![Text::new("AAAA", TextStyle::default().size(2))]);
        let prefix = WeightedText::from(Text::new("P", TextStyle::default().size(2)));
        let instructions =
            TestDrawer::default().prefix(prefix).max_line_length(6).start_column(1).repeat_prefix_on_wrap().draw(text);
        let expected = &[
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: "P".into(), font_size: 2 },
            Instruction::PrintText { content: "AA".into(), font_size: 2 },
            Instruction::MoveDown(2),
            Instruction::MoveToColumn(1),
            Instruction::PrintText { content: "P".into(), font_size: 2 },
            Instruction::PrintText { content: "AA".into(), font_size: 2 },
        ];
        assert_eq!(instructions, expected);
    }
}
