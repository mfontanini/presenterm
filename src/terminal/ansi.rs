use crate::markdown::{
    elements::{Line, Text},
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

    pub(crate) fn parse_lines<I, S>(self, lines: I) -> (Vec<Line>, TextStyle)
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
            output_lines.push(line);
            style = ending_style;
        }
        (output_lines, style)
    }
}

#[derive(Default)]
pub(crate) struct AnsiColorParser {
    starting_style: TextStyle,
}

impl AnsiColorParser {
    pub(crate) fn new(starting_style: TextStyle) -> Self {
        Self { starting_style }
    }

    fn parse_8bit(value: u16) -> Option<Color> {
        Color::from_8bit(value.try_into().unwrap_or(u8::MAX))
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
                Color::from_8bit(color.try_into().unwrap_or(u8::MAX))
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

    pub(crate) fn parse(self, mut codes: ParamsIter) -> TextStyle {
        let mut style = self.starting_style;
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
                    if let Some(color) = Self::parse_8bit(next - 30) {
                        style = style.fg_color(color);
                    }
                }
                40..=47 => {
                    if let Some(color) = Self::parse_8bit(next - 40) {
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
}

impl Perform for Handler {
    fn print(&mut self, c: char) {
        self.pending_text.content.push(c);
    }

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        if action == 'm' {
            self.save_pending_text();
            self.style = AnsiColorParser::new(self.style).parse(params.iter());
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
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::DarkRed)))
    )]
    #[case::standard_foreground2(
        "\x1b[31mhi", 
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::DarkRed)))
    )]
    #[case::rgb_foreground(
        "\x1b[38;2;3;4;5mhi", 
        Line::from(Text::new("hi", TextStyle::default().fg_color(Color::new(3, 4, 5))))
    )]
    #[case::standard_background1(
        "\x1b[48;5;1mhi", 
        Line::from(Text::new("hi", TextStyle::default().bg_color(Color::DarkRed)))
    )]
    #[case::standard_background2(
        "\x1b[41mhi", 
        Line::from(Text::new("hi", TextStyle::default().bg_color(Color::DarkRed)))
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
        assert_eq!(lines, vec![expected]);
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
        assert_eq!(lines, vec![expected]);
    }
}
