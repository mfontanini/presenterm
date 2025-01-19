use super::text_style::{Color, ParseColorError, TextStyle};
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
        let tag = node.as_tag().ok_or(ParseHtmlError::NoTags)?;
        if tag.name().as_bytes() != b"span" {
            return Err(ParseHtmlError::UnsupportedHtml);
        }
        let style = self.parse_attributes(tag.attributes())?;
        Ok(HtmlInline::OpenSpan { style })
    }

    fn parse_attributes(&self, attributes: &Attributes) -> Result<TextStyle, ParseHtmlError> {
        let mut style = TextStyle::default();
        for (name, value) in attributes.iter() {
            let value = value.unwrap_or(Cow::Borrowed(""));
            match name.as_ref() {
                "style" => self.parse_css_attribute(&value, &mut style)?,
                _ => {
                    if self.options.strict {
                        return Err(ParseHtmlError::UnsupportedTagAttribute(name.to_string()));
                    }
                }
            }
        }
        Ok(style)
    }

    fn parse_css_attribute(&self, attribute: &str, style: &mut TextStyle) -> Result<(), ParseHtmlError> {
        for attribute in attribute.split(';') {
            let attribute = attribute.trim();
            if attribute.is_empty() {
                continue;
            }
            let (key, value) = attribute.split_once(':').ok_or(ParseHtmlError::NoColonInAttribute)?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "color" => *style = style.fg_color(Self::parse_color(value)?),
                "background-color" => *style = style.bg_color(Self::parse_color(value)?),
                _ => {
                    if self.options.strict {
                        return Err(ParseHtmlError::UnsupportedCssAttribute(key.into()));
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_color(input: &str) -> Result<Color, ParseHtmlError> {
        if input.starts_with('#') {
            let color = input.strip_prefix('#').unwrap().parse()?;
            if matches!(color, Color::Rgb { .. }) { Ok(color) } else { Ok(input.parse()?) }
        } else {
            let color = input.parse::<Color>()?;
            if matches!(color, Color::Rgb { .. }) {
                Err(ParseHtmlError::InvalidColor("missing '#' in rgb color".into()))
            } else {
                Ok(color)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum HtmlInline {
    OpenSpan { style: TextStyle },
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
    use crate::markdown::text_style::Color;
    use rstest::rstest;

    #[test]
    fn parse() {
        let tag =
            HtmlParser::default().parse(r#"<span style="color: red; background-color: black">"#).expect("parse failed");
        let HtmlInline::OpenSpan { style } = tag else { panic!("not an open tag") };
        assert_eq!(style, TextStyle::default().bg_color(Color::Black).fg_color(Color::Red));
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
        let color: Color = HtmlParser::parse_color(input).expect("parse failed");
        assert_eq!(color, expected);
    }

    #[rstest]
    #[case::rgb("ff0000")]
    #[case::red("#red")]
    fn parse_invalid_color(#[case] input: &str) {
        HtmlParser::parse_color(input).expect_err("parse succeeded");
    }
}
