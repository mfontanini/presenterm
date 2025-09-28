use crate::{
    markdown::text_style::{Color, TextStyle},
    theme::raw::{ParseColorError, RawColor},
};
use std::{borrow::Cow, str, str::Utf8Error};
use tl::Attributes;

pub(crate) struct InlineHtmlParseOptions {
    pub(crate) strict: bool,
}

impl Default for InlineHtmlParseOptions {
    fn default() -> Self {
        Self { strict: true }
    }
}

#[derive(Default)]
pub(crate) struct InlineHtmlParser {
    options: InlineHtmlParseOptions,
}

impl InlineHtmlParser {
    pub(crate) fn parse(self, input: &str) -> Result<HtmlInline, ParseInlineHtmlError> {
        if input.starts_with("</") {
            if input.starts_with("</span") {
                return Ok(HtmlInline::CloseTag { tag: HtmlInlineTag::Span });
            } else if input.starts_with("</sup") {
                return Ok(HtmlInline::CloseTag { tag: HtmlInlineTag::Sup });
            } else {
                return Err(ParseInlineHtmlError::UnsupportedClosingTag(input.to_string()));
            }
        }
        let dom = tl::parse(input, Default::default())?;
        let top = dom.children().iter().next().ok_or(ParseInlineHtmlError::NoTags)?;
        let node = top.get(dom.parser()).expect("failed to get");
        let tag = node.as_tag().ok_or(ParseInlineHtmlError::NoTags)?;
        let (output_tag, base_style) = match tag.name().as_bytes() {
            b"span" => (HtmlInlineTag::Span, TextStyle::default()),
            b"sup" => (HtmlInlineTag::Sup, TextStyle::default().superscript()),
            _ => return Err(ParseInlineHtmlError::UnsupportedHtml),
        };
        let style = self.parse_attributes(tag.attributes())?;
        Ok(HtmlInline::OpenTag { style: style.merged(&base_style), tag: output_tag })
    }

    fn parse_attributes(&self, attributes: &Attributes) -> Result<TextStyle<RawColor>, ParseInlineHtmlError> {
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
                        return Err(ParseInlineHtmlError::UnsupportedTagAttribute(name.to_string()));
                    }
                }
            }
        }
        Ok(style)
    }

    fn parse_css_attribute(
        &self,
        attribute: &str,
        style: &mut TextStyle<RawColor>,
    ) -> Result<(), ParseInlineHtmlError> {
        for attribute in attribute.split(';') {
            let attribute = attribute.trim();
            if attribute.is_empty() {
                continue;
            }
            let (key, value) = attribute.split_once(':').ok_or(ParseInlineHtmlError::NoColonInAttribute)?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "color" => style.colors.foreground = Some(Self::parse_color(value)?),
                "background-color" => style.colors.background = Some(Self::parse_color(value)?),
                _ => {
                    if self.options.strict {
                        return Err(ParseInlineHtmlError::UnsupportedCssAttribute(key.into()));
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_color(input: &str) -> Result<RawColor, ParseInlineHtmlError> {
        if input.starts_with('#') {
            let color = input.strip_prefix('#').unwrap().parse()?;
            if matches!(color, RawColor::Color(Color::Rgb { .. })) { Ok(color) } else { Ok(input.parse()?) }
        } else {
            let color = input.parse::<RawColor>()?;
            if matches!(color, RawColor::Color(Color::Rgb { .. })) {
                Err(ParseInlineHtmlError::InvalidColor("missing '#' in rgb color".into()))
            } else {
                Ok(color)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum HtmlInline {
    OpenTag { style: TextStyle<RawColor>, tag: HtmlInlineTag },
    CloseTag { tag: HtmlInlineTag },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HtmlInlineTag {
    Span,
    Sup,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseInlineHtmlError {
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

    #[error("HTML can only contain span and sup tags")]
    UnsupportedHtml,

    #[error("unsupported tag attribute: {0}")]
    UnsupportedTagAttribute(String),

    #[error("unsupported closing tag: {0}")]
    UnsupportedClosingTag(String),
}

impl From<ParseColorError> for ParseInlineHtmlError {
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
        let tag = InlineHtmlParser::default()
            .parse(r#"<span style="color: red; background-color: black">"#)
            .expect("parse failed");
        let HtmlInline::OpenTag { style, tag: HtmlInlineTag::Span } = tag else { panic!("not an open tag") };
        assert_eq!(style, TextStyle::default().bg_color(Color::Black).fg_color(Color::Red));
    }

    #[test]
    fn parse_sup() {
        let tag = InlineHtmlParser::default().parse(r#"<sup>"#).expect("parse failed");
        let HtmlInline::OpenTag { style, tag: HtmlInlineTag::Sup } = tag else { panic!("not an open tag") };
        assert_eq!(style, TextStyle::default().superscript());
    }

    #[test]
    fn parse_class() {
        let tag = InlineHtmlParser::default().parse(r#"<span class="foo">"#).expect("parse failed");
        let HtmlInline::OpenTag { style, tag: HtmlInlineTag::Span } = tag else { panic!("not an open tag") };
        assert_eq!(
            style,
            TextStyle::default()
                .bg_color(RawColor::BackgroundClass("foo".into()))
                .fg_color(RawColor::ForegroundClass("foo".into()))
        );
    }

    #[rstest]
    #[case::span("</span>", HtmlInlineTag::Span)]
    #[case::sup("</sup>", HtmlInlineTag::Sup)]
    fn parse_end_tag(#[case] input: &str, #[case] tag: HtmlInlineTag) {
        let inline = InlineHtmlParser::default().parse(input).expect("parse failed");
        assert_eq!(inline, HtmlInline::CloseTag { tag });
    }

    #[rstest]
    #[case::invalid_start_tag("<div>")]
    #[case::invalid_end_tag("</div>")]
    #[case::invalid_attribute("<span foo=\"bar\">")]
    #[case::invalid_attribute("<span style=\"bleh: 42\"")]
    #[case::invalid_color("<span style=\"color: 42\"")]
    fn parse_invalid_html(#[case] input: &str) {
        InlineHtmlParser::default().parse(input).expect_err("parse succeeded");
    }

    #[rstest]
    #[case::rgb("#ff0000", Color::Rgb{r: 255, g: 0, b: 0})]
    #[case::red("red", Color::Red)]
    fn parse_color(#[case] input: &str, #[case] expected: Color) {
        let color = InlineHtmlParser::parse_color(input).expect("parse failed");
        assert_eq!(color, expected.into());
    }

    #[rstest]
    #[case::rgb("ff0000")]
    #[case::red("#red")]
    fn parse_invalid_color(#[case] input: &str) {
        InlineHtmlParser::parse_color(input).expect_err("parse succeeded");
    }
}
