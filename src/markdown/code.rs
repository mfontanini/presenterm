use super::elements::{
    Highlight, HighlightGroup, Percent, PercentParseError, Snippet, SnippetAttributes, SnippetLanguage,
};
use comrak::nodes::NodeCodeBlock;
use strum::EnumDiscriminants;

pub(crate) type ParseResult<T> = Result<T, CodeBlockParseError>;

pub(crate) struct CodeBlockParser;

impl CodeBlockParser {
    pub(crate) fn parse(code_block: &NodeCodeBlock) -> ParseResult<Snippet> {
        let (language, attributes) = Self::parse_block_info(&code_block.info)?;
        let code = Snippet { contents: code_block.literal.clone(), language, attributes };
        Ok(code)
    }

    fn parse_block_info(input: &str) -> ParseResult<(SnippetLanguage, SnippetAttributes)> {
        let (language, input) = Self::parse_language(input);
        let attributes = Self::parse_attributes(input)?;
        if attributes.auto_render && !language.supports_auto_render() {
            return Err(CodeBlockParseError::UnsupportedAttribute(language, "rendering"));
        }
        if attributes.width.is_some() && !attributes.auto_render {
            return Err(CodeBlockParseError::NotRenderSnippet("width"));
        }
        Ok((language, attributes))
    }

    fn parse_language(input: &str) -> (SnippetLanguage, &str) {
        let token = Self::next_identifier(input);
        // this always returns `Ok` given we fall back to `Unknown` if we don't know the language.
        let language = token.parse().expect("language parsing");
        let rest = &input[token.len()..];
        (language, rest)
    }

    fn parse_attributes(mut input: &str) -> ParseResult<SnippetAttributes> {
        let mut attributes = SnippetAttributes::default();
        let mut processed_attributes = Vec::new();
        while let (Some(attribute), rest) = Self::parse_attribute(input)? {
            let discriminant = AttributeDiscriminants::from(&attribute);
            if processed_attributes.contains(&discriminant) {
                return Err(CodeBlockParseError::DuplicateAttribute("duplicate attribute"));
            }
            match attribute {
                Attribute::LineNumbers => attributes.line_numbers = true,
                Attribute::Exec => attributes.execute = true,
                Attribute::AutoRender => attributes.auto_render = true,
                Attribute::HighlightedLines(lines) => attributes.highlight_groups = lines,
                Attribute::Width(width) => attributes.width = Some(width),
            };
            processed_attributes.push(discriminant);
            input = rest;
        }
        if attributes.highlight_groups.is_empty() {
            attributes.highlight_groups.push(HighlightGroup::new(vec![Highlight::All]));
        }
        Ok(attributes)
    }

    fn parse_attribute(input: &str) -> ParseResult<(Option<Attribute>, &str)> {
        let input = Self::skip_whitespace(input);
        let (attribute, input) = match input.chars().next() {
            Some('+') => {
                let token = Self::next_identifier(&input[1..]);
                let attribute = match token {
                    "line_numbers" => Attribute::LineNumbers,
                    "exec" => Attribute::Exec,
                    "render" => Attribute::AutoRender,
                    token if token.starts_with("width:") => {
                        let value = input.split_once("+width:").unwrap().1;
                        let (width, input) = Self::parse_width(value)?;
                        return Ok((Some(Attribute::Width(width)), input));
                    }
                    _ => return Err(CodeBlockParseError::InvalidToken(Self::next_identifier(input).into())),
                };
                (Some(attribute), &input[token.len() + 1..])
            }
            Some('{') => {
                let (lines, input) = Self::parse_highlight_groups(&input[1..])?;
                (Some(Attribute::HighlightedLines(lines)), input)
            }
            Some(_) => return Err(CodeBlockParseError::InvalidToken(Self::next_identifier(input).into())),
            None => (None, input),
        };
        Ok((attribute, input))
    }

    fn parse_highlight_groups(input: &str) -> ParseResult<(Vec<HighlightGroup>, &str)> {
        use CodeBlockParseError::InvalidHighlightedLines;
        let Some((head, tail)) = input.split_once('}') else {
            return Err(InvalidHighlightedLines("no enclosing '}'".into()));
        };
        let head = head.trim();
        if head.is_empty() {
            return Ok((Vec::new(), tail));
        }

        let mut highlight_groups = Vec::new();
        for group in head.split('|') {
            let group = Self::parse_highlight_group(group)?;
            highlight_groups.push(group);
        }
        Ok((highlight_groups, tail))
    }

    fn parse_highlight_group(input: &str) -> ParseResult<HighlightGroup> {
        let mut highlights = Vec::new();
        for piece in input.split(',') {
            let piece = piece.trim();
            if piece == "all" {
                highlights.push(Highlight::All);
                continue;
            }
            match piece.split_once('-') {
                Some((left, right)) => {
                    let left = Self::parse_number(left)?;
                    let right = Self::parse_number(right)?;
                    let right = right
                        .checked_add(1)
                        .ok_or_else(|| CodeBlockParseError::InvalidHighlightedLines(format!("{right} is too large")))?;
                    highlights.push(Highlight::Range(left..right));
                }
                None => {
                    let number = Self::parse_number(piece)?;
                    highlights.push(Highlight::Single(number));
                }
            }
        }
        Ok(HighlightGroup::new(highlights))
    }

    fn parse_number(input: &str) -> ParseResult<u16> {
        input
            .trim()
            .parse()
            .map_err(|_| CodeBlockParseError::InvalidHighlightedLines(format!("not a number: '{input}'")))
    }

    fn parse_width(input: &str) -> ParseResult<(Percent, &str)> {
        let end_index = input.find(' ').unwrap_or(input.len());
        let value = input[0..end_index].parse().map_err(CodeBlockParseError::InvalidWidth)?;
        Ok((value, &input[end_index..]))
    }

    fn skip_whitespace(input: &str) -> &str {
        input.trim_start_matches(' ')
    }

    fn next_identifier(input: &str) -> &str {
        match input.split_once(' ') {
            Some((token, _)) => token,
            None => input,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum CodeBlockParseError {
    #[error("invalid code attribute: {0}")]
    InvalidToken(String),

    #[error("invalid highlighted lines: {0}")]
    InvalidHighlightedLines(String),

    #[error("invalid width: {0}")]
    InvalidWidth(PercentParseError),

    #[error("duplicate attribute: {0}")]
    DuplicateAttribute(&'static str),

    #[error("language {0:?} does not support {1}")]
    UnsupportedAttribute(SnippetLanguage, &'static str),

    #[error("attribute {0} can only be set in +render blocks")]
    NotRenderSnippet(&'static str),
}

#[derive(EnumDiscriminants)]
enum Attribute {
    LineNumbers,
    Exec,
    AutoRender,
    HighlightedLines(Vec<HighlightGroup>),
    Width(Percent),
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;
    use Highlight::*;

    fn parse_language(input: &str) -> SnippetLanguage {
        let (language, _) = CodeBlockParser::parse_block_info(input).expect("parse failed");
        language
    }

    fn try_parse_attributes(input: &str) -> Result<SnippetAttributes, CodeBlockParseError> {
        let (_, attributes) = CodeBlockParser::parse_block_info(input)?;
        Ok(attributes)
    }

    fn parse_attributes(input: &str) -> SnippetAttributes {
        try_parse_attributes(input).expect("parse failed")
    }

    #[test]
    fn unknown_language() {
        assert_eq!(parse_language("potato"), SnippetLanguage::Unknown("potato".to_string()));
    }

    #[test]
    fn no_attributes() {
        assert_eq!(parse_language("rust"), SnippetLanguage::Rust);
    }

    #[test]
    fn one_attribute() {
        let attributes = parse_attributes("bash +exec");
        assert!(attributes.execute);
        assert!(!attributes.line_numbers);
    }

    #[test]
    fn two_attributes() {
        let attributes = parse_attributes("bash +exec +line_numbers");
        assert!(attributes.execute);
        assert!(attributes.line_numbers);
    }

    #[test]
    fn invalid_attributes() {
        CodeBlockParser::parse_block_info("bash +potato").unwrap_err();
        CodeBlockParser::parse_block_info("bash potato").unwrap_err();
    }

    #[rstest]
    #[case::no_end("{")]
    #[case::number_no_end("{42")]
    #[case::comma_nothing("{42,")]
    #[case::brace_comma("{,}")]
    #[case::range_no_end("{42-")]
    #[case::range_end("{42-}")]
    #[case::too_many_ranges("{42-3-5}")]
    #[case::range_comma("{42-,")]
    #[case::too_large("{65536}")]
    #[case::too_large_end("{1-65536}")]
    fn invalid_line_highlights(#[case] input: &str) {
        let input = format!("bash {input}");
        CodeBlockParser::parse_block_info(&input).expect_err("parsed successfully");
    }

    #[test]
    fn highlight_none() {
        let attributes = parse_attributes("bash {}");
        assert_eq!(attributes.highlight_groups, &[HighlightGroup::new(vec![Highlight::All])]);
    }

    #[test]
    fn highlight_specific_lines() {
        let attributes = parse_attributes("bash {   1, 2  , 3   }");
        assert_eq!(attributes.highlight_groups, &[HighlightGroup::new(vec![Single(1), Single(2), Single(3)])]);
    }

    #[test]
    fn highlight_line_range() {
        let attributes = parse_attributes("bash {   1, 2-4,6 ,  all , 10 - 12  }");
        assert_eq!(
            attributes.highlight_groups,
            &[HighlightGroup::new(vec![Single(1), Range(2..5), Single(6), All, Range(10..13)])]
        );
    }

    #[test]
    fn multiple_groups() {
        let attributes = parse_attributes("bash {1-3,5  |6-9}");
        assert_eq!(attributes.highlight_groups.len(), 2);
        assert_eq!(attributes.highlight_groups[0], HighlightGroup::new(vec![Range(1..4), Single(5)]));
        assert_eq!(attributes.highlight_groups[1], HighlightGroup::new(vec![Range(6..10)]));
    }

    #[test]
    fn parse_width() {
        let attributes = parse_attributes("mermaid +width:50% +render");
        assert!(attributes.auto_render);
        assert_eq!(attributes.width, Some(Percent(50)));
    }

    #[test]
    fn invalid_width() {
        try_parse_attributes("mermaid +width:50%% +render").expect_err("parse succeeded");
        try_parse_attributes("mermaid +width: +render").expect_err("parse succeeded");
        try_parse_attributes("mermaid +width:50%").expect_err("parse succeeded");
    }
}
