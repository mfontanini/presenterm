use crate::{
    markdown::{elements::Line, text_style::Color},
    terminal::printer::TerminalCommand,
};
use std::fmt::Debug;
use unicode_width::UnicodeWidthStr;

pub(crate) mod slide_horizontal;

#[derive(Clone, Debug)]
pub(crate) enum TransitionDirection {
    Next,
    Previous,
}

pub(crate) trait AnimateTransition {
    type Frame: AnimationFrame + Debug;

    fn build_frame(&self, frame: usize, direction: TransitionDirection) -> Self::Frame;
    fn total_frames(&self) -> usize;
}

pub(crate) trait AnimationFrame {
    fn build_commands(&self) -> Vec<TerminalCommand>;
}

#[derive(Debug)]
pub(crate) struct LinesFrame {
    pub(crate) lines: Vec<Line>,
    pub(crate) background_color: Option<Color>,
}

impl LinesFrame {
    fn skip_whitespace(mut text: &str) -> (&str, usize, usize) {
        let mut trimmed_before = 0;
        while let Some(' ') = text.chars().next() {
            text = &text[1..];
            trimmed_before += 1;
        }
        let mut trimmed_after = 0;
        let mut rev = text.chars().rev();
        while let Some(' ') = rev.next() {
            text = &text[..text.len() - 1];
            trimmed_after += 1;
        }
        (text, trimmed_before, trimmed_after)
    }
}

impl AnimationFrame for LinesFrame {
    fn build_commands(&self) -> Vec<TerminalCommand> {
        use TerminalCommand::*;
        let mut commands = vec![];
        if let Some(color) = self.background_color {
            commands.push(SetBackgroundColor(color));
        }
        commands.push(ClearScreen);
        for (row, line) in self.lines.iter().enumerate() {
            let mut column = 0;
            let mut is_in_column = false;
            let mut is_in_row = false;
            for chunk in &line.0 {
                let (text, white_before, white_after) = match chunk.style.colors.background {
                    Some(_) => (chunk.content.as_str(), 0, 0),
                    None => Self::skip_whitespace(&chunk.content),
                };
                // If this is an empty line just skip it
                if text.is_empty() {
                    column += chunk.content.width();
                    is_in_column = false;
                    continue;
                }
                if !is_in_row {
                    commands.push(MoveToRow(row as u16));
                    is_in_row = true;
                }
                if white_before > 0 {
                    column += white_before;
                    is_in_column = false;
                }
                if !is_in_column {
                    commands.push(MoveToColumn(column as u16));
                    is_in_column = true;
                }
                commands.push(PrintText { content: text, style: chunk.style });
                column += text.width();
                if white_after > 0 {
                    column += white_after;
                    is_in_column = false;
                }
            }
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::elements::Text;

    #[test]
    fn commands() {
        let animation = LinesFrame {
            lines: vec![
                Line(vec![Text::from("  hi  "), Text::from("bye"), Text::from("s")]),
                Line(vec![Text::from("hello"), Text::from(" wor"), Text::from("s")]),
            ],
            background_color: Some(Color::Red),
        };
        let commands = animation.build_commands();
        use TerminalCommand::*;
        let expected = &[
            SetBackgroundColor(Color::Red),
            ClearScreen,
            MoveToRow(0),
            MoveToColumn(2),
            PrintText { content: "hi", style: Default::default() },
            MoveToColumn(6),
            PrintText { content: "bye", style: Default::default() },
            PrintText { content: "s", style: Default::default() },
            MoveToRow(1),
            MoveToColumn(0),
            PrintText { content: "hello", style: Default::default() },
            MoveToColumn(6),
            PrintText { content: "wor", style: Default::default() },
            PrintText { content: "s", style: Default::default() },
        ];
        assert_eq!(commands, expected);
    }
}
