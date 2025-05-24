use crate::markdown::text_style::{Color, TextAttribute, TextStyle};
use std::{borrow::Cow, fmt};

pub(crate) enum HtmlText {
    Plain(String),
    Styled { text: String, style: String },
}

impl HtmlText {
    pub(crate) fn new(text: &str, style: &TextStyle, font_size: FontSize) -> Self {
        let mut text = text.to_string();
        if style == &TextStyle::default() {
            return Self::Plain(text);
        }
        let mut css_styles = Vec::new();
        let mut text_decorations = Vec::new();
        for attr in style.iter_attributes() {
            match attr {
                TextAttribute::Bold => css_styles.push(Cow::Borrowed("font-weight: bold")),
                TextAttribute::Italics => css_styles.push(Cow::Borrowed("font-style: italic")),
                TextAttribute::Strikethrough => text_decorations.push(Cow::Borrowed("line-through")),
                TextAttribute::Underlined => text_decorations.push(Cow::Borrowed("underline")),
                TextAttribute::Superscript => text = format!("<sup>{text}</sup>"),
                TextAttribute::ForegroundColor(color) => {
                    let color = color_to_html(&color);
                    css_styles.push(format!("color: {color}").into());
                }
                TextAttribute::BackgroundColor(color) => {
                    let color = color_to_html(&color);
                    css_styles.push(format!("background-color: {color}").into());
                }
            };
        }
        if !text_decorations.is_empty() {
            let text_decoration = text_decorations.join(" ");
            css_styles.push(format!("text-decoration: {text_decoration}").into());
        }
        if style.size > 1 {
            let font_size = font_size.scale(style.size);
            css_styles.push(format!("font-size: {font_size}").into());
        }
        let css_style = css_styles.join("; ");
        Self::Styled { text, style: css_style }
    }
}

impl fmt::Display for HtmlText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plain(text) => write!(f, "{text}"),
            Self::Styled { text, style } => write!(f, "<span style=\"{style}\">{text}</span>"),
        }
    }
}

pub(crate) enum FontSize {
    Pixels(u16),
}

impl FontSize {
    fn scale(&self, size: u8) -> String {
        match self {
            Self::Pixels(scale) => format!("{}px", scale * size as u16),
        }
    }
}

pub(crate) fn color_to_html(color: &Color) -> String {
    match color {
        Color::Black => "#000000".into(),
        Color::DarkGrey => "#5a5a5a".into(),
        Color::Red => "#ff0000".into(),
        Color::DarkRed => "#8b0000".into(),
        Color::Green => "#00ff00".into(),
        Color::DarkGreen => "#006400".into(),
        Color::Yellow => "#ffff00".into(),
        Color::DarkYellow => "#8b8000".into(),
        Color::Blue => "#0000ff".into(),
        Color::DarkBlue => "#00008b".into(),
        Color::Magenta => "#ff00ff".into(),
        Color::DarkMagenta => "#8b008b".into(),
        Color::Cyan => "#00ffff".into(),
        Color::DarkCyan => "#008b8b".into(),
        Color::White => "#ffffff".into(),
        Color::Grey => "#808080".into(),
        Color::Rgb { r, g, b } => format!("#{r:02x}{g:02x}{b:02x}"),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::none(TextStyle::default(), "")]
    #[case::bold(TextStyle::default().bold(), "font-weight: bold")]
    #[case::italics(TextStyle::default().italics(), "font-style: italic")]
    #[case::bold_italics(TextStyle::default().bold().italics(), "font-weight: bold; font-style: italic")]
    #[case::strikethrough(TextStyle::default().strikethrough(), "text-decoration: line-through")]
    #[case::underlined(TextStyle::default().underlined(), "text-decoration: underline")]
    #[case::strikethrough_underlined(
        TextStyle::default().strikethrough().underlined(),
        "text-decoration: line-through underline"
    )]
    #[case::foreground_color(TextStyle::default().fg_color(Color::new(1,2,3)), "color: #010203")]
    #[case::background_color(TextStyle::default().bg_color(Color::new(1,2,3)), "background-color: #010203")]
    #[case::font_size(TextStyle::default().size(3), "font-size: 6px")]
    fn html_text(#[case] style: TextStyle, #[case] expected_style: &str) {
        let html_text = HtmlText::new("", &style, FontSize::Pixels(2));
        let style = match &html_text {
            HtmlText::Plain(_) => "",
            HtmlText::Styled { style, .. } => style,
        };
        assert_eq!(style, expected_style);
    }

    #[test]
    fn render_span() {
        let html_text = HtmlText::new("hi", &TextStyle::default().bold(), FontSize::Pixels(1));
        let rendered = html_text.to_string();
        assert_eq!(rendered, "<span style=\"font-weight: bold\">hi</span>");
    }
}
