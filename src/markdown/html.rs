use super::text_style::{Color, TextStyle};
use crate::theme::raw::{ParseColorError, RawColor};
use std::{borrow::Cow, str, str::Utf8Error};
use tl::Attributes;

pub(crate) struct HtmlParseOptions {
    pub(crate) strict: bool,
}

impl Default for HtmlParseOptions {
    fn default() -> Self {
        Self { strict: true }
    }
}

#[derive(Default)]
pub(crate) struct HtmlParser {
    options: HtmlParseOptions,
}

impl HtmlParser {
    pub(crate) fn parse(self, input: &str) -> Result<HtmlInline, ParseHtmlError> {
        if input.starts_with("</") {
            if input.starts_with("</span") {
                return Ok(HtmlInline::CloseSpan);
            } else {
                return Err(ParseHtmlError::UnsupportedClosingTag(input.to_string()));
            }
        }
        let dom = tl::parse(input, Default::default())?;
        let top = dom.children().iter().next().ok_or(ParseHtmlError::NoTags)?;
        let node = top.get(dom.parser()).expect("failed to get");

        // Check for inline pause
        if let Some(comment) = node.as_comment() {
            let bytes = comment.as_bytes().trim_ascii();

            if bytes.len() >= 7 && bytes.starts_with(b"<!--") && bytes.ends_with(b"-->") {
                let content = &bytes[4..bytes.len() - 3].trim_ascii();

                if content != b"pause" {
                    return Err(ParseHtmlError::UnsupportedHtml);
                }

                return match str::from_utf8(content) {
                    Ok(_) => Ok(HtmlInline::PauseCommand),
                    Err(parse_err) => Err(ParseHtmlError::NotUtf8(parse_err)),
                };
            } else {
                return Err(ParseHtmlError::UnsupportedHtml);
            }
        }

        let tag = node.as_tag().ok_or(ParseHtmlError::NoTags)?;
        if tag.name().as_bytes() != b"span" {
            return Err(ParseHtmlError::UnsupportedHtml);
        }
        let style = self.parse_attributes(tag.attributes())?;
        Ok(HtmlInline::OpenSpan { style })
    }

    fn parse_attributes(&self, attributes: &Attributes) -> Result<TextStyle<RawColor>, ParseHtmlError> {
        let mut style = TextStyle::default();
        for (name, value) in attributes.iter() {
            let value = value.unwrap_or(Cow::Borrowed(""));
            match name.as_ref() {
                "style" => self.parse_css_attribute(&value, &mut style)?,
                "class" => {
                    style = style.fg_color(RawColor::ForegroundClass(value.to_string()));
                    style = style.bg_color(RawColor::BackgroundClass(value.to_string()));
                }
                _ => {
                    if self.options.strict {
                        return Err(ParseHtmlError::UnsupportedTagAttribute(name.to_string()));
                    }
                }
            }
        }
        Ok(style)
    }

    fn parse_css_attribute(&self, attribute: &str, style: &mut TextStyle<RawColor>) -> Result<(), ParseHtmlError> {
        for attribute in attribute.split(';') {
            let attribute = attribute.trim();
            if attribute.is_empty() {
                continue;
            }
            let (key, value) = attribute.split_once(':').ok_or(ParseHtmlError::NoColonInAttribute)?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "color" => style.colors.foreground = Some(Self::parse_color(value)?),
                "background-color" => style.colors.background = Some(Self::parse_color(value)?),
                _ => {
                    if self.options.strict {
                        return Err(ParseHtmlError::UnsupportedCssAttribute(key.into()));
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_color(input: &str) -> Result<RawColor, ParseHtmlError> {
        if input.starts_with('#') {
            let color = input.strip_prefix('#').unwrap().parse()?;
            if matches!(color, RawColor::Color(Color::Rgb { .. })) { Ok(color) } else { Ok(input.parse()?) }
        } else {
            let color = input.parse::<RawColor>()?;
            if matches!(color, RawColor::Color(Color::Rgb { .. })) {
                Err(ParseHtmlError::InvalidColor("missing '#' in rgb color".into()))
            } else {
                Ok(color)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum HtmlInline {
    PauseCommand,
    OpenSpan { style: TextStyle<RawColor> },
    CloseSpan,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseHtmlError {
    #[error("parsing html failed: {0}")]
    ParsingHtml(#[from] tl::ParseError),

    #[error("no html tags found")]
    NoTags,

    #[error("non utf8 content: {0}")]
    NotUtf8(#[from] Utf8Error),

    #[error("attribute has no ':'")]
    NoColonInAttribute,

    #[error("invalid color: {0}")]
    InvalidColor(String),

    #[error("invalid css attribute: {0}")]
    UnsupportedCssAttribute(String),

    #[error("HTML can only contain span tags")]
    UnsupportedHtml,

    #[error("unsupported tag attribute: {0}")]
    UnsupportedTagAttribute(String),

    #[error("unsupported closing tag: {0}")]
    UnsupportedClosingTag(String),
}

impl From<ParseColorError> for ParseHtmlError {
    fn from(e: ParseColorError) -> Self {
        Self::InvalidColor(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn parse_style() {
        let tag =
            HtmlParser::default().parse(r#"<span style="color: red; background-color: black">"#).expect("parse failed");
        let HtmlInline::OpenSpan { style } = tag else { panic!("not an open tag") };
        assert_eq!(style, TextStyle::default().bg_color(Color::Black).fg_color(Color::Red));
    }

    #[test]
    fn parse_class() {
        let tag = HtmlParser::default().parse(r#"<span class="foo">"#).expect("parse failed");
        let HtmlInline::OpenSpan { style } = tag else { panic!("not an open tag") };
        assert_eq!(
            style,
            TextStyle::default()
                .bg_color(RawColor::BackgroundClass("foo".into()))
                .fg_color(RawColor::ForegroundClass("foo".into()))
        );
    }

    #[test]
    fn parse_end_tag() {
        let tag = HtmlParser::default().parse("</span>").expect("parse failed");
        assert!(matches!(tag, HtmlInline::CloseSpan));
    }

    #[rstest]
    #[case::invalid_start_tag("<div>")]
    #[case::invalid_end_tag("</div>")]
    #[case::invalid_attribute("<span foo=\"bar\">")]
    #[case::invalid_attribute("<span style=\"bleh: 42\"")]
    #[case::invalid_color("<span style=\"color: 42\"")]
    fn parse_invalid_html(#[case] input: &str) {
        HtmlParser::default().parse(input).expect_err("parse succeeded");
    }

    #[rstest]
    #[case::rgb("#ff0000", Color::Rgb{r: 255, g: 0, b: 0})]
    #[case::red("red", Color::Red)]
    fn parse_color(#[case] input: &str, #[case] expected: Color) {
        let color = HtmlParser::parse_color(input).expect("parse failed");
        assert_eq!(color, expected.into());
    }

    #[rstest]
    #[case::rgb("ff0000")]
    #[case::red("#red")]
    fn parse_invalid_color(#[case] input: &str) {
        HtmlParser::parse_color(input).expect_err("parse succeeded");
    }
}
