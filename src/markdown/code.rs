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
                Attribute::LineNumbers => attributes.line_numbers = true,
                Attribute::Exec => attributes.execute = true,
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
            Some(_) => return Err(CodeBlockParseError::InvalidToken(Self::next_identifier(input).into())),
            None => (None, input),
        };
        Ok((attribute, input))
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
}

enum Attribute {
    LineNumbers,
    Exec,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unknown_language() {
        let (language, _) = CodeBlockParser::parse_block_info("potato").expect("parse failed");
        assert_eq!(language, CodeLanguage::Unknown);
    }

    #[test]
    fn no_attributes() {
        let (language, _) = CodeBlockParser::parse_block_info("rust").expect("parse failed");
        assert_eq!(language, CodeLanguage::Rust);
    }

    #[test]
    fn one_attribute() {
        let (_, attributes) = CodeBlockParser::parse_block_info("bash +exec").expect("parse failed");
        assert!(attributes.execute);
        assert!(!attributes.line_numbers);
    }

    #[test]
    fn two_attributes() {
        let (_, attributes) = CodeBlockParser::parse_block_info("bash +exec +line_numbers").expect("parse failed");
        assert!(attributes.execute);
        assert!(attributes.line_numbers);
    }

    #[test]
    fn invalid_attributes() {
        CodeBlockParser::parse_block_info("bash +potato").unwrap_err();
        CodeBlockParser::parse_block_info("bash potato").unwrap_err();
    }
}
