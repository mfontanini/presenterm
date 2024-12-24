use crate::{
    markdown::{
        elements::{Line, Text},
        text::WeightedLine,
    },
    style::{Color, TextStyle},
};
use ansi_parser::{AnsiParser, AnsiSequence, Output};

pub(crate) struct AnsiSplitter {
    lines: Vec<WeightedLine>,
    current_line: Line,
    current_style: TextStyle,
}

impl AnsiSplitter {
    pub(crate) fn new(current_style: TextStyle) -> Self {
        Self { lines: Default::default(), current_line: Default::default(), current_style }
    }

    pub(crate) fn split_lines(mut self, lines: &[String]) -> (Vec<WeightedLine>, TextStyle) {
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

impl GraphicsCode<'_> {
    fn update(&self, style: &mut TextStyle) {
        let codes = self.0;
        match codes {
            [] | [0] => *style = Default::default(),
            [1] => *style = style.bold(),
            [3] => *style = style.italics(),
            [4] => *style = style.underlined(),
            [9] => *style = style.strikethrough(),
            [39] => style.colors.foreground = None,
            [49] => style.colors.background = None,
            [value] | [1, value] => match value {
                30..=37 => {
                    if let Some(color) = Self::as_standard_color(value - 30) {
                        *style = style.fg_color(color);
                    }
                }
                40..=47 => {
                    if let Some(color) = Self::as_standard_color(value - 40) {
                        *style = style.bg_color(color);
                    }
                }
                _ => (),
            },
            [38, 2, r, g, b] => {
                *style = style.fg_color(Color::new(*r, *g, *b));
            }
            [38, 5, value] => {
                if let Some(color) = Self::parse_color(*value) {
                    *style = style.fg_color(color);
                }
            }
            [48, 2, r, g, b] => {
                *style = style.bg_color(Color::new(*r, *g, *b));
            }
            [48, 5, value] => {
                if let Some(color) = Self::parse_color(*value) {
                    *style = style.bg_color(color);
                }
            }
            _ => (),
        };
    }

    fn parse_color(value: u8) -> Option<Color> {
        match value {
            0..=15 => Self::as_standard_color(value),
            16..=231 => {
                let mapping = [0, 95, 95 + 40, 95 + 80, 95 + 120, 95 + 160];
                let mut value = value - 16;
                let b = (value % 6) as usize;
                value /= 6;
                let g = (value % 6) as usize;
                value /= 6;
                let r = (value % 6) as usize;
                Some(Color::new(mapping[r], mapping[g], mapping[b]))
            }
            _ => None,
        }
    }

    fn as_standard_color(value: u8) -> Option<Color> {
        let color = match value {
            0 | 8 => Color::Black,
            1 | 9 => Color::Red,
            2 | 10 => Color::Green,
            3 | 11 => Color::Yellow,
            4 | 12 => Color::Blue,
            5 | 13 => Color::Magenta,
            6 | 14 => Color::Cyan,
            7 | 15 => Color::White,
            _ => return None,
        };
        Some(color)
    }
}
