use crate::markdown::{
    elements::{Line, Text},
    text::WeightedLine,
    text_style::{Color, TextStyle},
};
use std::mem;
use vte::{ParamsIter, Parser, Perform};

pub(crate) struct AnsiParser {
    starting_style: TextStyle,
}

impl AnsiParser {
    pub(crate) fn new(current_style: TextStyle) -> Self {
        Self { starting_style: current_style }
    }

    pub(crate) fn parse_lines<I, S>(self, lines: I) -> (Vec<WeightedLine>, TextStyle)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut output_lines = Vec::new();
        let mut style = self.starting_style;
        for line in lines {
            let mut handler = Handler::new(style);
            let mut parser = Parser::new();
            parser.advance(&mut handler, line.as_ref().as_bytes());

            let (line, ending_style) = handler.into_parts();
            output_lines.push(line.into());
            style = ending_style;
        }
        (output_lines, style)
    }
}

struct Handler {
    line: Line,
    pending_text: Text,
    style: TextStyle,
}

impl Handler {
    fn new(style: TextStyle) -> Self {
        Self { line: Default::default(), pending_text: Default::default(), style }
    }

    fn into_parts(mut self) -> (Line, TextStyle) {
        self.save_pending_text();
        (self.line, self.style)
    }

    fn save_pending_text(&mut self) {
        if !self.pending_text.content.is_empty() {
            self.line.0.push(mem::take(&mut self.pending_text));
        }
    }

    fn parse_standard_color(value: u16) -> Option<Color> {
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

    fn parse_color(iter: &mut ParamsIter) -> Option<Color> {
        match iter.next()? {
            [2] => {
                let r = iter.next()?.first()?;
                let g = iter.next()?.first()?;
                let b = iter.next()?.first()?;
                Self::try_build_rgb_color(*r, *g, *b)
            }
            [5] => {
                let color = *iter.next()?.first()?;
                match color {
                    0..=15 => Self::parse_standard_color(color),
                    16..=231 => {
                        let mapping = [0, 95, 95 + 40, 95 + 80, 95 + 120, 95 + 160];
                        let mut value = color - 16;
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
            _ => None,
        }
    }

    fn try_build_rgb_color(r: u16, g: u16, b: u16) -> Option<Color> {
        let r = r.try_into().ok()?;
        let g = g.try_into().ok()?;
        let b = b.try_into().ok()?;
        Some(Color::new(r, g, b))
    }

    fn update_style(&self, mut codes: ParamsIter) -> TextStyle {
        let mut style = self.style;
        loop {
            let Some(&[next]) = codes.next() else {
                break;
            };
            match next {
                0 => style = Default::default(),
                1 => style = style.bold(),
                3 => style = style.italics(),
                4 => style = style.underlined(),
                9 => style = style.strikethrough(),
                39 => {
                    style.colors.foreground = None;
                }
                49 => {
                    style.colors.background = None;
                }
                30..=37 => {
                    if let Some(color) = Self::parse_standard_color(next - 30) {
                        style = style.fg_color(color);
                    }
                }
                40..=47 => {
                    if let Some(color) = Self::parse_standard_color(next - 40) {
                        style = style.bg_color(color);
                    }
                }
                38 => {
                    if let Some(color) = Self::parse_color(&mut codes) {
                        style = style.fg_color(color);
                    }
                }
                48 => {
                    if let Some(color) = Self::parse_color(&mut codes) {
                        style = style.bg_color(color);
                    }
                }
                _ => (),
            };
        }
        style
    }
}

impl Perform for Handler {
    fn print(&mut self, c: char) {
        self.pending_text.content.push(c);
    }

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        if action == 'm' {
            self.save_pending_text();
            self.style = self.update_style(params.iter());
            self.pending_text.style = self.style;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::text("hi", Line::from("hi"))]
    #[case::single_attribute("\x1b[1mhi", Line::from(Text::new("hi", TextStyle::default().bold())))]
    #[case::two_attributes("\x1b[1;3mhi", Line::from(Text::new("hi", TextStyle::default().bold().italics())))]
    #[case::three_attributes("\x1b[1;3;4mhi", Line::from(Text::new("hi", TextStyle::default().bold().italics().underlined())))]
    #[case::four_attributes(
        "\x1b[1;3;4;9mhi", 
        Line::from(Text::new("hi", TextStyle::default().bold().italics().underlined().strikethrough()))
    )]
    #[case::standard_foreground1(
        "\x1b[38;5;1mhi", 
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::Red)))
    )]
    #[case::standard_foreground2(
        "\x1b[31mhi", 
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::Red)))
    )]
    #[case::rgb_foreground(
        "\x1b[38;2;3;4;5mhi", 
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::new(3, 4, 5))))
    )]
    #[case::standard_background1(
        "\x1b[48;5;1mhi", 
        Line::from(Text::new("hi", TextStyle::default().bg_color(Color::Red)))
    )]
    #[case::standard_background2(
        "\x1b[41mhi", 
        Line::from(Text::new("hi", TextStyle::default().bg_color(Color::Red)))
    )]
    #[case::rgb_background(
        "\x1b[48;2;3;4;5mhi", 
        Line::from(Text::new("hi", TextStyle::default().bg_color(Color::new(3, 4, 5))))
    )]
    #[case::accumulate(
        "\x1b[1mhi\x1b[3mbye", 
        Line(vec![
            Text::new("hi", TextStyle::default().bold()),
            Text::new("bye", TextStyle::default().bold().italics())
        ])
    )]
    #[case::reset(
        "\x1b[1mhi\x1b[0;3mbye", 
        Line(vec![
            Text::new("hi", TextStyle::default().bold()),
            Text::new("bye", TextStyle::default().italics())
        ])
    )]
    #[case::different_action(
        "\x1b[01m\x1b[Khi",
        Line::from(Text::new("hi", TextStyle::default().bold()))
    )]
    fn parse_single(#[case] input: &str, #[case] expected: Line) {
        let splitter = AnsiParser::new(Default::default());
        let (lines, _) = splitter.parse_lines([input]);
        assert_eq!(lines, vec![expected.into()]);
    }

    #[rstest]
    #[case::reset_all("\x1b[0mhi", Line::from("hi"))]
    #[case::reset_foreground(
        "\x1b[39mhi", 
        Line::from(
            Text::new(
                "hi", 
                TextStyle::default()
                    .bold()
                    .italics()
                    .underlined()
                    .strikethrough()
                    .bg_color(Color::Black)
            )
        )
    )]
    #[case::reset_background(
        "\x1b[49mhi", 
        Line::from(
            Text::new(
                "hi", 
                TextStyle::default()
                    .bold()
                    .italics()
                    .underlined()
                    .strikethrough()
                    .fg_color(Color::Red)
            )
        )
    )]
    fn resets(#[case] input: &str, #[case] expected: Line) {
        let style = TextStyle::default()
            .bold()
            .italics()
            .underlined()
            .strikethrough()
            .fg_color(Color::Red)
            .bg_color(Color::Black);
        let splitter = AnsiParser::new(style);
        let (lines, _) = splitter.parse_lines([input]);
        assert_eq!(lines, vec![expected.into()]);
    }
}
