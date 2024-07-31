use crate::{
    markdown::{
        elements::{Text, TextBlock},
        text::WeightedTextBlock,
    },
    style::{Color, TextStyle},
};
use ansi_parser::{AnsiParser, AnsiSequence, Output};

pub(crate) struct AnsiSplitter {
    lines: Vec<WeightedTextBlock>,
    current_line: TextBlock,
    current_style: TextStyle,
}

impl AnsiSplitter {
    pub(crate) fn new(current_style: TextStyle) -> Self {
        Self { lines: Default::default(), current_line: Default::default(), current_style }
    }

    pub(crate) fn split_lines(mut self, lines: &[String]) -> (Vec<WeightedTextBlock>, TextStyle) {
        for line in lines {
            for p in line.ansi_parse() {
                match p {
                    Output::TextBlock(text) => {
                        self.current_line.0.push(Text::new(text, self.current_style));
                    }
                    Output::Escape(s) => self.handle_escape(&s),
                }
            }
            let current_line = std::mem::take(&mut self.current_line);
            self.lines.push(current_line.into());
        }

        (self.lines, self.current_style)
    }

    fn handle_escape(&mut self, s: &AnsiSequence) {
        match s {
            AnsiSequence::SetGraphicsMode(code) => {
                let code = GraphicsCode(code);
                code.update(&mut self.current_style);
            }
            AnsiSequence::EraseDisplay => {
                self.lines.clear();
                self.current_line.0.clear();
            }
            _ => (),
        }
    }
}

struct GraphicsCode<'a>(&'a [u8]);

impl<'a> GraphicsCode<'a> {
    fn update(&self, style: &mut TextStyle) {
        // RGB mode
        let codes = self.0;
        if codes.starts_with(&[38, 2]) || codes.starts_with(&[48, 2]) {
            if codes.len() == 5 {
                let color = Color::new(codes[2], codes[3], codes[4]);
                if codes[0] == 38 {
                    style.colors.foreground = Some(color);
                } else {
                    style.colors.background = Some(color);
                }
            }
            return;
        }
        for value in codes {
            match value {
                0 => *style = TextStyle::default(),
                1 => *style = (*style).bold(),
                3 => *style = (*style).italics(),
                4 => *style = (*style).underlined(),
                9 => *style = (*style).strikethrough(),
                30 => style.colors.foreground = Some(Color::Black),
                40 => style.colors.background = Some(Color::Black),
                31 => style.colors.foreground = Some(Color::Red),
                41 => style.colors.background = Some(Color::Red),
                32 => style.colors.foreground = Some(Color::Green),
                42 => style.colors.background = Some(Color::Green),
                33 => style.colors.foreground = Some(Color::Yellow),
                43 => style.colors.background = Some(Color::Yellow),
                34 => style.colors.foreground = Some(Color::Blue),
                44 => style.colors.background = Some(Color::Blue),
                35 => style.colors.foreground = Some(Color::Magenta),
                45 => style.colors.background = Some(Color::Magenta),
                36 => style.colors.foreground = Some(Color::Cyan),
                46 => style.colors.background = Some(Color::Cyan),
                37 => style.colors.foreground = Some(Color::White),
                47 => style.colors.background = Some(Color::White),
                39 => style.colors.foreground = None,
                49 => style.colors.background = None,
                _ => (),
            }
        }
    }
}
