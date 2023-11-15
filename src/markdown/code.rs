use super::elements::{Code, CodeAttributes, CodeLanguage};
use comrak::nodes::NodeCodeBlock;

pub(crate) type ParseResult<T> = Result<T, CodeBlockParseError>;

pub(crate) struct CodeBlockParser;

impl CodeBlockParser {
    pub(crate) fn parse(code_block: &NodeCodeBlock) -> ParseResult<Code> {
        let (language, attributes) = Self::parse_block_info(&code_block.info)?;
        let code = Code { contents: code_block.literal.clone(), language, attributes };
        Ok(code)
    }

    fn parse_block_info(input: &str) -> ParseResult<(CodeLanguage, CodeAttributes)> {
        let (language, input) = Self::parse_language(input);
        let attributes = Self::parse_attributes(input)?;
        if attributes.execute && !language.supports_execution() {
            return Err(CodeBlockParseError::ExecutionNotSupported(language));
        }
        Ok((language, attributes))
    }

    fn parse_language(input: &str) -> (CodeLanguage, &str) {
        let token = Self::next_identifier(input);
        use CodeLanguage::*;
        let language = match token {
            "ada" => Ada,
            "asp" => Asp,
            "awk" => Awk,
            "c" => C,
            "cmake" => CMake,
            "crontab" => Crontab,
            "csharp" => CSharp,
            "clojure" => Clojure,
            "cpp" | "c++" => Cpp,
            "css" => Css,
            "d" => DLang,
            "docker" => Docker,
            "dotenv" => Dotenv,
            "elixir" => Elixir,
            "elm" => Elm,
            "erlang" => Erlang,
            "go" => Go,
            "haskell" => Haskell,
            "html" => Html,
            "java" => Java,
            "javascript" | "js" => JavaScript,
            "json" => Json,
            "kotlin" => Kotlin,
            "latex" => Latex,
            "lua" => Lua,
            "make" => Makefile,
            "markdown" => Markdown,
            "ocaml" => OCaml,
            "perl" => Perl,
            "php" => Php,
            "protobuf" => Protobuf,
            "puppet" => Puppet,
            "python" => Python,
            "r" => R,
            "rust" => Rust,
            "scala" => Scala,
            "shell" => Shell("sh".into()),
            interpreter @ ("bash" | "sh" | "zsh" | "fish") => Shell(interpreter.into()),
            "sql" => Sql,
            "svelte" => Svelte,
            "swift" => Swift,
            "terraform" => Terraform,
            "typescript" | "ts" => TypeScript,
            "xml" => Xml,
            "yaml" => Yaml,
            "vue" => Vue,
            "zig" => Zig,
            _ => Unknown,
        };
        let rest = &input[token.len()..];
        (language, rest)
    }

    fn parse_attributes(mut input: &str) -> ParseResult<CodeAttributes> {
        let mut attributes = CodeAttributes::default();
        while let (Some(attribute), rest) = Self::parse_attribute(input)? {
            match attribute {
                Attribute::LineNumbers => {
                    if attributes.line_numbers {
                        return Err(CodeBlockParseError::DuplicateAttribute("line_numbers"));
                    }
                    attributes.line_numbers = true
                }

                Attribute::Exec => {
                    if attributes.execute {
                        return Err(CodeBlockParseError::DuplicateAttribute("exec"));
                    }
                    attributes.execute = true
                }
                Attribute::HighlightedLines(lines) => {
                    if attributes.highlighted_lines.is_some() {
                        return Err(CodeBlockParseError::DuplicateAttribute("{..}"));
                    }
                    attributes.highlighted_lines = Some(lines)
                }
            };
            input = rest;
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
                    _ => return Err(CodeBlockParseError::InvalidToken(Self::next_identifier(input).into())),
                };
                (Some(attribute), &input[token.len() + 1..])
            }
            Some('{') => {
                let (lines, input) = Self::parse_highlighted_lines(&input[1..])?;
                (Some(Attribute::HighlightedLines(lines)), input)
            }
            Some(_) => return Err(CodeBlockParseError::InvalidToken(Self::next_identifier(input).into())),
            None => (None, input),
        };
        Ok((attribute, input))
    }

    fn parse_highlighted_lines(input: &str) -> ParseResult<(Vec<u16>, &str)> {
        let Some((head, tail)) = input.split_once('}') else {
            return Err(CodeBlockParseError::InvalidHighlightedLines("no enclosing '}'".into()));
        };
        let mut lines = Vec::new();
        for piece in head.split(',') {
            let piece = piece.trim();
            match piece.split_once('-') {
                Some((left, right)) => {
                    let left = Self::parse_number(left)?;
                    let right = Self::parse_number(right)?;
                    lines.extend(left..=right);
                }
                None => {
                    let number = Self::parse_number(piece)?;
                    lines.push(number);
                }
            }
        }
        Ok((lines, tail))
    }

    fn parse_number(input: &str) -> ParseResult<u16> {
        input
            .trim()
            .parse()
            .map_err(|_| CodeBlockParseError::InvalidHighlightedLines(format!("not a number: '{input}'")))
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

    #[error("duplicate attribute: {0}")]
    DuplicateAttribute(&'static str),

    #[error("language {0:?} does not support execution")]
    ExecutionNotSupported(CodeLanguage),
}

enum Attribute {
    LineNumbers,
    Exec,
    HighlightedLines(Vec<u16>),
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

    fn parse_language(input: &str) -> CodeLanguage {
        let (language, _) = CodeBlockParser::parse_block_info(input).expect("parse failed");
        language
    }

    fn parse_attributes(input: &str) -> CodeAttributes {
        let (_, attributes) = CodeBlockParser::parse_block_info(input).expect("parse failed");
        attributes
    }

    #[test]
    fn unknown_language() {
        assert_eq!(parse_language("potato"), CodeLanguage::Unknown);
    }

    #[test]
    fn no_attributes() {
        assert_eq!(parse_language("rust"), CodeLanguage::Rust);
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
    fn highlight_specific_lines() {
        let attributes = parse_attributes("bash {   1, 2  , 3   }");
        assert_eq!(attributes.highlighted_lines, Some(vec![1, 2, 3]));
    }

    #[test]
    fn highlight_line_range() {
        let attributes = parse_attributes("bash {   1, 2-4,5 ,10 - 12  }");
        assert_eq!(attributes.highlighted_lines, Some(vec![1, 2, 3, 4, 5, 10, 11, 12]));
    }
}
