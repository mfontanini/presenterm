use super::{
    highlighting::{LanguageHighlighter, StyledTokens},
    padding::NumberPadder,
};
use crate::{
    markdown::{
        elements::{Percent, PercentParseError},
        text::{WeightedLine, WeightedText},
        text_style::{Color, TextStyle},
    },
    presentation::ChunkMutator,
    render::{
        operation::{AsRenderOperations, BlockLine, RenderOperation},
        properties::WindowSize,
    },
    theme::{Alignment, CodeBlockStyle},
};
use serde::Deserialize;
use std::{cell::RefCell, convert::Infallible, fmt::Write, ops::Range, path::PathBuf, rc::Rc, str::FromStr};
use strum::{EnumDiscriminants, EnumIter};
use unicode_width::UnicodeWidthStr;

pub(crate) struct SnippetSplitter<'a> {
    style: &'a CodeBlockStyle,
    hidden_line_prefix: Option<&'a str>,
}

impl<'a> SnippetSplitter<'a> {
    pub(crate) fn new(style: &'a CodeBlockStyle, hidden_line_prefix: Option<&'a str>) -> Self {
        Self { style, hidden_line_prefix }
    }

    pub(crate) fn split(&self, code: &Snippet) -> Vec<SnippetLine> {
        let mut lines = Vec::new();
        let horizontal_padding = self.style.padding.horizontal;
        let vertical_padding = self.style.padding.vertical;
        if vertical_padding > 0 {
            lines.push(SnippetLine::empty());
        }
        self.push_lines(code, horizontal_padding, &mut lines);
        if vertical_padding > 0 {
            lines.push(SnippetLine::empty());
        }
        lines
    }

    fn push_lines(&self, code: &Snippet, horizontal_padding: u8, lines: &mut Vec<SnippetLine>) {
        if code.contents.is_empty() {
            return;
        }

        let padding = " ".repeat(horizontal_padding as usize);
        let padder = NumberPadder::new(code.visible_lines(self.hidden_line_prefix).count());
        for (index, line) in code.visible_lines(self.hidden_line_prefix).enumerate() {
            let mut line = line.replace('\t', "    ");
            let mut prefix = padding.clone();
            if code.attributes.line_numbers {
                let line_number = index + 1;
                prefix.push_str(&padder.pad_right(line_number));
                prefix.push(' ');
            }
            line.push('\n');
            let line_number = Some(index as u16 + 1);
            lines.push(SnippetLine { prefix, code: line, right_padding_length: padding.len() as u16, line_number });
        }
    }
}

pub(crate) struct SnippetLine {
    pub(crate) prefix: String,
    pub(crate) code: String,
    pub(crate) right_padding_length: u16,
    pub(crate) line_number: Option<u16>,
}

impl SnippetLine {
    pub(crate) fn empty() -> Self {
        Self { prefix: String::new(), code: "\n".into(), right_padding_length: 0, line_number: None }
    }

    pub(crate) fn width(&self) -> usize {
        self.prefix.width() + self.code.width() + self.right_padding_length as usize
    }

    pub(crate) fn highlight(
        &self,
        code_highlighter: &mut LanguageHighlighter,
        block_style: &CodeBlockStyle,
        font_size: u8,
    ) -> WeightedLine {
        let mut line = code_highlighter.highlight_line(&self.code, block_style);
        line.apply_style(&TextStyle::default().size(font_size));
        line.into()
    }

    pub(crate) fn dim(&self, dim_style: &TextStyle) -> WeightedLine {
        let output = vec![StyledTokens { style: *dim_style, tokens: &self.code }.apply_style()];
        output.into()
    }

    pub(crate) fn dim_prefix(&self, dim_style: &TextStyle) -> WeightedText {
        let text = StyledTokens { style: *dim_style, tokens: &self.prefix }.apply_style();
        text.into()
    }
}

#[derive(Debug)]
pub(crate) struct HighlightContext {
    pub(crate) groups: Vec<HighlightGroup>,
    pub(crate) current: usize,
    pub(crate) block_length: u16,
    pub(crate) alignment: Alignment,
}

#[derive(Debug)]
pub(crate) struct HighlightedLine {
    pub(crate) prefix: WeightedText,
    pub(crate) right_padding_length: u16,
    pub(crate) highlighted: WeightedLine,
    pub(crate) not_highlighted: WeightedLine,
    pub(crate) line_number: Option<u16>,
    pub(crate) context: Rc<RefCell<HighlightContext>>,
    pub(crate) block_color: Option<Color>,
}

impl AsRenderOperations for HighlightedLine {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        let context = self.context.borrow();
        let group = &context.groups[context.current];
        let needs_highlight = self.line_number.map(|number| group.contains(number)).unwrap_or_default();
        // TODO: Cow<str>?
        let text = match needs_highlight {
            true => self.highlighted.clone(),
            false => self.not_highlighted.clone(),
        };
        vec![
            RenderOperation::RenderBlockLine(BlockLine {
                prefix: self.prefix.clone(),
                right_padding_length: self.right_padding_length,
                repeat_prefix_on_wrap: false,
                text,
                block_length: context.block_length,
                alignment: context.alignment,
                block_color: self.block_color,
            }),
            RenderOperation::RenderLineBreak,
        ]
    }
}

#[derive(Debug)]
pub(crate) struct HighlightMutator {
    context: Rc<RefCell<HighlightContext>>,
}

impl HighlightMutator {
    pub(crate) fn new(context: Rc<RefCell<HighlightContext>>) -> Self {
        Self { context }
    }
}

impl ChunkMutator for HighlightMutator {
    fn mutate_next(&self) -> bool {
        let mut context = self.context.borrow_mut();
        if context.current == context.groups.len() - 1 {
            false
        } else {
            context.current += 1;
            true
        }
    }

    fn mutate_previous(&self) -> bool {
        let mut context = self.context.borrow_mut();
        if context.current == 0 {
            false
        } else {
            context.current -= 1;
            true
        }
    }

    fn reset_mutations(&self) {
        self.context.borrow_mut().current = 0;
    }

    fn apply_all_mutations(&self) {
        let mut context = self.context.borrow_mut();
        context.current = context.groups.len() - 1;
    }

    fn mutations(&self) -> (usize, usize) {
        let context = self.context.borrow();
        (context.current, context.groups.len())
    }
}

pub(crate) type ParseResult<T> = Result<T, SnippetBlockParseError>;

pub(crate) struct SnippetParser;

impl SnippetParser {
    pub(crate) fn parse(info: String, code: String) -> ParseResult<Snippet> {
        let (language, attributes) = Self::parse_block_info(&info)?;
        let code = Snippet { contents: code, language, attributes };
        Ok(code)
    }

    fn parse_block_info(input: &str) -> ParseResult<(SnippetLanguage, SnippetAttributes)> {
        let (language, input) = Self::parse_language(input);
        let attributes = Self::parse_attributes(input)?;
        if attributes.width.is_some() && !matches!(attributes.representation, SnippetRepr::Render) {
            return Err(SnippetBlockParseError::NotRenderSnippet("width"));
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
            let discriminant = SnippetAttributeDiscriminants::from(&attribute);
            if processed_attributes.contains(&discriminant) {
                return Err(SnippetBlockParseError::DuplicateAttribute("duplicate attribute"));
            }
            use SnippetAttribute::*;
            match attribute {
                ExecReplace(_) | Image | Render if attributes.representation != SnippetRepr::Snippet => {
                    return Err(SnippetBlockParseError::MultipleRepresentation);
                }
                LineNumbers => attributes.line_numbers = true,
                Exec(spec) => {
                    if !matches!(attributes.execution, SnippetExec::AcquireTerminal(_)) {
                        attributes.execution = SnippetExec::Exec(spec);
                    }
                }
                AutoExec(spec) => {
                    if !matches!(attributes.execution, SnippetExec::AcquireTerminal(_)) {
                        attributes.execution = SnippetExec::AutoExec(spec);
                    }
                }
                ExecPty(spec) => {
                    if !matches!(attributes.execution, SnippetExec::AcquireTerminal(_)) {
                        attributes.execution = SnippetExec::ExecPty(spec);
                    }
                }
                ExecReplace(spec) => {
                    attributes.representation = SnippetRepr::ExecReplace;
                    attributes.execution = SnippetExec::Exec(spec);
                }
                Id(id) => {
                    attributes.id = Some(id);
                }
                Validate(spec) => {
                    if matches!(attributes.execution, SnippetExec::None) {
                        attributes.execution = SnippetExec::Validate(spec);
                    }
                }
                Image => {
                    attributes.representation = SnippetRepr::Image;
                    attributes.execution = SnippetExec::Exec(Default::default());
                }
                Render => attributes.representation = SnippetRepr::Render,
                AcquireTerminal(spec) => attributes.execution = SnippetExec::AcquireTerminal(spec),
                NoBackground => attributes.no_background = true,
                HighlightedLines(lines) => attributes.highlight_groups = lines,
                Width(width) => attributes.width = Some(width),
                ExpectedExecutionResult(result) => attributes.expected_execution_result = result,
            };
            processed_attributes.push(discriminant);
            input = rest;
        }
        if attributes.highlight_groups.is_empty() {
            attributes.highlight_groups.push(HighlightGroup::new(vec![Highlight::All]));
        }
        Ok(attributes)
    }

    fn parse_attribute(input: &str) -> ParseResult<(Option<SnippetAttribute>, &str)> {
        let input = Self::skip_whitespace(input);
        let (attribute, input) = match input.chars().next() {
            Some('+') => {
                let token = Self::next_identifier(&input[1..]);
                let attribute = match token {
                    "line_numbers" => SnippetAttribute::LineNumbers,
                    "exec" => SnippetAttribute::Exec(SnippetExecutorSpec::default()),
                    "auto_exec" => SnippetAttribute::AutoExec(SnippetExecutorSpec::default()),
                    "exec_replace" => SnippetAttribute::ExecReplace(SnippetExecutorSpec::default()),
                    "validate" => SnippetAttribute::Validate(SnippetExecutorSpec::default()),
                    "image" => SnippetAttribute::Image,
                    "render" => SnippetAttribute::Render,
                    "no_background" => SnippetAttribute::NoBackground,
                    "acquire_terminal" => SnippetAttribute::AcquireTerminal(SnippetExecutorSpec::default()),
                    "pty" => SnippetAttribute::ExecPty(SnippetExecutorSpec::default()),
                    other => {
                        let (attribute, parameter) = other
                            .split_once(':')
                            .ok_or_else(|| SnippetBlockParseError::InvalidToken(Self::next_identifier(input).into()))?;
                        match attribute {
                            "exec" => SnippetAttribute::Exec(SnippetExecutorSpec::Alternative(parameter.to_string())),
                            "auto_exec" => {
                                SnippetAttribute::AutoExec(SnippetExecutorSpec::Alternative(parameter.to_string()))
                            }
                            "exec_replace" => {
                                SnippetAttribute::ExecReplace(SnippetExecutorSpec::Alternative(parameter.to_string()))
                            }
                            "pty" => SnippetAttribute::ExecPty(SnippetExecutorSpec::Alternative(parameter.to_string())),
                            "id" => SnippetAttribute::Id(parameter.to_string()),
                            "validate" => {
                                SnippetAttribute::Validate(SnippetExecutorSpec::Alternative(parameter.to_string()))
                            }
                            "acquire_terminal" => SnippetAttribute::AcquireTerminal(SnippetExecutorSpec::Alternative(
                                parameter.to_string(),
                            )),
                            "width" => {
                                let width = parameter.parse().map_err(SnippetBlockParseError::InvalidWidth)?;
                                SnippetAttribute::Width(width)
                            }
                            "expect" => match parameter {
                                "success" => {
                                    SnippetAttribute::ExpectedExecutionResult(ExpectedSnippetExecutionResult::Success)
                                }
                                "failure" | "fail" => {
                                    SnippetAttribute::ExpectedExecutionResult(ExpectedSnippetExecutionResult::Failure)
                                }
                                _ => {
                                    return Err(SnippetBlockParseError::InvalidToken(
                                        Self::next_identifier(input).into(),
                                    ));
                                }
                            },
                            _ => return Err(SnippetBlockParseError::InvalidToken(Self::next_identifier(input).into())),
                        }
                    }
                };
                (Some(attribute), &input[token.len() + 1..])
            }
            Some('{') => {
                let (lines, input) = Self::parse_highlight_groups(&input[1..])?;
                (Some(SnippetAttribute::HighlightedLines(lines)), input)
            }
            Some(_) => return Err(SnippetBlockParseError::InvalidToken(Self::next_identifier(input).into())),
            None => (None, input),
        };
        Ok((attribute, input))
    }

    fn parse_highlight_groups(input: &str) -> ParseResult<(Vec<HighlightGroup>, &str)> {
        use SnippetBlockParseError::InvalidHighlightedLines;
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
                    let right = right.checked_add(1).ok_or_else(|| {
                        SnippetBlockParseError::InvalidHighlightedLines(format!("{right} is too large"))
                    })?;
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
            .map_err(|_| SnippetBlockParseError::InvalidHighlightedLines(format!("not a number: '{input}'")))
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
pub enum SnippetBlockParseError {
    #[error("invalid code attribute: {0}")]
    InvalidToken(String),

    #[error("invalid highlighted lines: {0}")]
    InvalidHighlightedLines(String),

    #[error("invalid width: {0}")]
    InvalidWidth(PercentParseError),

    #[error("duplicate attribute: {0}")]
    DuplicateAttribute(&'static str),

    #[error("+exec_replace +image and +render can't be used together ")]
    MultipleRepresentation,

    #[error("attribute {0} can only be set in +render blocks")]
    NotRenderSnippet(&'static str),
}

#[derive(EnumDiscriminants)]
enum SnippetAttribute {
    LineNumbers,
    Exec(SnippetExecutorSpec),
    AutoExec(SnippetExecutorSpec),
    ExecReplace(SnippetExecutorSpec),
    ExecPty(SnippetExecutorSpec),
    Validate(SnippetExecutorSpec),
    Image,
    Render,
    HighlightedLines(Vec<HighlightGroup>),
    Width(Percent),
    NoBackground,
    AcquireTerminal(SnippetExecutorSpec),
    ExpectedExecutionResult(ExpectedSnippetExecutionResult),
    Id(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum SnippetExecutorSpec {
    #[default]
    Default,
    Alternative(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ExpectedSnippetExecutionResult {
    #[default]
    Success,
    Failure,
}

/// A code snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snippet {
    /// The snippet itself.
    pub(crate) contents: String,

    /// The programming language this snippet is written in.
    pub(crate) language: SnippetLanguage,

    /// The attributes used for snippet.
    pub(crate) attributes: SnippetAttributes,
}

impl Snippet {
    pub(crate) fn visible_lines<'a, 'b>(
        &'a self,
        hidden_line_prefix: Option<&'b str>,
    ) -> impl Iterator<Item = &'a str> + 'b
    where
        'a: 'b,
    {
        self.contents.lines().filter(move |line| !hidden_line_prefix.is_some_and(|prefix| line.starts_with(prefix)))
    }

    pub(crate) fn executable_contents(&self, hidden_line_prefix: Option<&str>) -> String {
        if let Some(prefix) = hidden_line_prefix {
            self.contents.lines().fold(String::new(), |mut output, line| {
                let line = line.strip_prefix(prefix).unwrap_or(line);
                let _ = writeln!(output, "{line}");
                output
            })
        } else {
            self.contents.to_owned()
        }
    }
}

/// The language of a code snippet.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, PartialOrd, Ord)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum SnippetLanguage {
    Ada,
    Asp,
    Awk,
    Bash,
    BatchFile,
    C,
    CMake,
    Crontab,
    CSharp,
    Clojure,
    Cpp,
    Css,
    D2,
    DLang,
    Diff,
    Docker,
    Dotenv,
    Elixir,
    Elm,
    Erlang,
    File,
    Fish,
    FSharp,
    Go,
    GraphQL,
    Haskell,
    Html,
    Java,
    JavaScript,
    Json,
    Jsonnet,
    Julia,
    Kotlin,
    Latex,
    Lua,
    Makefile,
    Mermaid,
    Markdown,
    Nix,
    Nushell,
    OCaml,
    Perl,
    Php,
    Protobuf,
    Puppet,
    Python,
    R,
    Racket,
    Ruby,
    Rust,
    RustScript,
    Scala,
    Shell,
    Sql,
    Swift,
    Svelte,
    Tcl,
    Terraform,
    Toml,
    TypeScript,
    Typst,
    Unknown(String),
    Xml,
    Yaml,
    Verilog,
    Vue,
    Zig,
    Zsh,
}

crate::utils::impl_deserialize_from_str!(SnippetLanguage);

impl FromStr for SnippetLanguage {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use SnippetLanguage::*;
        let language = match s.to_lowercase().as_str() {
            "ada" => Ada,
            "asp" => Asp,
            "awk" => Awk,
            "bash" => Bash,
            "c" => C,
            "cmake" => CMake,
            "crontab" => Crontab,
            "csharp" => CSharp,
            "clojure" => Clojure,
            "cpp" | "c++" => Cpp,
            "css" => Css,
            "d2" => D2,
            "d" => DLang,
            "diff" => Diff,
            "docker" => Docker,
            "dotenv" => Dotenv,
            "elixir" => Elixir,
            "elm" => Elm,
            "erlang" => Erlang,
            "file" => File,
            "fish" => Fish,
            "fsharp" => FSharp,
            "go" => Go,
            "graphql" => GraphQL,
            "haskell" => Haskell,
            "html" => Html,
            "java" => Java,
            "javascript" | "js" => JavaScript,
            "json" => Json,
            "jsonnet" => Jsonnet,
            "julia" => Julia,
            "kotlin" => Kotlin,
            "latex" => Latex,
            "lua" => Lua,
            "make" => Makefile,
            "markdown" => Markdown,
            "mermaid" => Mermaid,
            "nix" => Nix,
            "nushell" | "nu" => Nushell,
            "ocaml" => OCaml,
            "perl" => Perl,
            "php" => Php,
            "protobuf" => Protobuf,
            "puppet" => Puppet,
            "python" => Python,
            "r" => R,
            "racket" => Racket,
            "ruby" => Ruby,
            "rust" => Rust,
            "rust-script" => RustScript,
            "scala" => Scala,
            "shell" | "sh" => Shell,
            "sql" => Sql,
            "svelte" => Svelte,
            "swift" => Swift,
            "tcl" => Tcl,
            "terraform" => Terraform,
            "toml" => Toml,
            "typescript" | "ts" => TypeScript,
            "typst" => Typst,
            "xml" => Xml,
            "yaml" => Yaml,
            "verilog" => Verilog,
            "vue" => Vue,
            "zig" => Zig,
            "zsh" => Zsh,
            other => Unknown(other.to_string()),
        };
        Ok(language)
    }
}

/// Attributes for code snippets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SnippetAttributes {
    /// The way the snippet should be represented.
    pub(crate) representation: SnippetRepr,

    /// The way the snippet should be executed.
    pub(crate) execution: SnippetExec,

    /// Whether the snippet should show line numbers.
    pub(crate) line_numbers: bool,

    /// The groups of lines to highlight.
    pub(crate) highlight_groups: Vec<HighlightGroup>,

    /// The width of the generated image.
    ///
    /// Only valid for +render snippets.
    pub(crate) width: Option<Percent>,

    /// Whether to add no background to a snippet.
    pub(crate) no_background: bool,

    /// The expected execution result for a snippet.
    pub(crate) expected_execution_result: ExpectedSnippetExecutionResult,

    /// The identifier for a snippet.
    pub(crate) id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum SnippetRepr {
    #[default]
    Snippet,
    Image,
    Render,
    ExecReplace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum SnippetExec {
    #[default]
    None,
    Exec(SnippetExecutorSpec),
    AutoExec(SnippetExecutorSpec),
    ExecPty(SnippetExecutorSpec),
    AcquireTerminal(SnippetExecutorSpec),
    Validate(SnippetExecutorSpec),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HighlightGroup(Vec<Highlight>);

impl HighlightGroup {
    pub(crate) fn new(highlights: Vec<Highlight>) -> Self {
        Self(highlights)
    }

    pub(crate) fn contains(&self, line_number: u16) -> bool {
        for higlight in &self.0 {
            match higlight {
                Highlight::All => return true,
                Highlight::Single(number) if number == &line_number => return true,
                Highlight::Range(range) if range.contains(&line_number) => return true,
                _ => continue,
            };
        }
        false
    }
}

/// A highlighted set of lines
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Highlight {
    All,
    Single(u16),
    Range(Range<u16>),
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExternalFile {
    pub(crate) path: PathBuf,
    pub(crate) language: SnippetLanguage,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
}

#[cfg(test)]
mod test {
    use super::*;
    use Highlight::*;
    use rstest::rstest;

    fn parse_language(input: &str) -> SnippetLanguage {
        let (language, _) = SnippetParser::parse_block_info(input).expect("parse failed");
        language
    }

    fn try_parse_attributes(input: &str) -> Result<SnippetAttributes, SnippetBlockParseError> {
        let (_, attributes) = SnippetParser::parse_block_info(input)?;
        Ok(attributes)
    }

    fn parse_attributes(input: &str) -> SnippetAttributes {
        try_parse_attributes(input).expect("parse failed")
    }

    #[test]
    fn code_with_line_numbers() {
        let total_lines = 11;
        let input_lines = "hi\n".repeat(total_lines);
        let code = Snippet {
            contents: input_lines,
            language: SnippetLanguage::Unknown("".to_string()),
            attributes: SnippetAttributes { line_numbers: true, ..Default::default() },
        };
        let lines = SnippetSplitter::new(&Default::default(), None).split(&code);
        assert_eq!(lines.len(), total_lines);

        let mut lines = lines.into_iter().enumerate();
        // 0..=9
        for (index, line) in lines.by_ref().take(9) {
            let line_number = index + 1;
            assert_eq!(&line.prefix, &format!(" {line_number} "));
        }
        // 10..
        for (index, line) in lines {
            let line_number = index + 1;
            assert_eq!(&line.prefix, &format!("{line_number} "));
        }
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
        assert_eq!(attributes.execution, SnippetExec::Exec(Default::default()));
        assert!(!attributes.line_numbers);
    }

    #[test]
    fn two_attributes() {
        let attributes = parse_attributes("bash +exec +line_numbers");
        assert_eq!(attributes.execution, SnippetExec::Exec(Default::default()));
        assert!(attributes.line_numbers);
    }

    #[test]
    fn acquire_terminal() {
        let attributes = parse_attributes("bash +acquire_terminal +exec");
        assert_eq!(attributes.execution, SnippetExec::AcquireTerminal(Default::default()));
        assert_eq!(attributes.representation, SnippetRepr::Snippet);
        assert!(!attributes.line_numbers);
    }

    #[test]
    fn image() {
        let attributes = parse_attributes("bash +image +exec");
        assert_eq!(attributes.execution, SnippetExec::Exec(Default::default()));
        assert_eq!(attributes.representation, SnippetRepr::Image);
        assert!(!attributes.line_numbers);
    }

    #[test]
    fn invalid_attributes() {
        SnippetParser::parse_block_info("bash +potato").unwrap_err();
        SnippetParser::parse_block_info("bash potato").unwrap_err();
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
        SnippetParser::parse_block_info(&input).expect_err("parsed successfully");
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
        assert_eq!(attributes.representation, SnippetRepr::Render);
        assert_eq!(attributes.width, Some(Percent(50)));
    }

    #[test]
    fn invalid_width() {
        try_parse_attributes("mermaid +width:50%% +render").expect_err("parse succeeded");
        try_parse_attributes("mermaid +width: +render").expect_err("parse succeeded");
        try_parse_attributes("mermaid +width:50%").expect_err("parse succeeded");
    }

    #[test]
    fn code_visible_lines() {
        let contents = r##"# fn main() {
println!("Hello world");
# // The prefix is # .
# }
"##
        .to_string();

        let expected = vec!["println!(\"Hello world\");"];
        let code = Snippet { contents, language: SnippetLanguage::Rust, attributes: Default::default() };
        assert_eq!(expected, code.visible_lines(Some("# ")).collect::<Vec<_>>());
    }

    #[test]
    fn code_executable_contents() {
        let contents = r##"# fn main() {
println!("Hello world");
# // The prefix is # .
# }
"##
        .to_string();

        let expected = r##"fn main() {
println!("Hello world");
// The prefix is # .
}
"##
        .to_string();

        let code = Snippet { contents, language: SnippetLanguage::Rust, attributes: Default::default() };
        assert_eq!(expected, code.executable_contents(Some("# ")));
    }

    #[test]
    fn tabs_in_snippet() {
        let snippet = Snippet { contents: "\thi".into(), language: SnippetLanguage::C, attributes: Default::default() };
        let lines = SnippetSplitter::new(&Default::default(), None).split(&snippet);
        assert_eq!(lines[0].code, "    hi\n");
    }

    #[rstest]
    #[case::exec("bash +exec:foo", SnippetExecutorSpec::Alternative("foo".to_string()))]
    #[case::exec_and_more("bash +exec:foo +line_numbers", SnippetExecutorSpec::Alternative("foo".to_string()))]
    #[case::exec_replace("bash +exec_replace:foo", SnippetExecutorSpec::Alternative("foo".to_string()))]
    #[case::exec_replace_and_more("bash +exec_replace:foo +line_numbers", SnippetExecutorSpec::Alternative("foo".into()))]
    fn alternative_executor(#[case] input: &str, #[case] spec: SnippetExecutorSpec) {
        let attributes = parse_attributes(input);
        assert_eq!(attributes.execution, SnippetExec::Exec(spec));
    }

    #[test]
    fn acquire_terminal_alternative() {
        let attributes = parse_attributes("bash +acquire_terminal:foo");
        assert_eq!(attributes.execution, SnippetExec::AcquireTerminal(SnippetExecutorSpec::Alternative("foo".into())));
    }

    #[rstest]
    #[case::success("expect:success", ExpectedSnippetExecutionResult::Success)]
    #[case::failure("expect:failure", ExpectedSnippetExecutionResult::Failure)]
    #[case::fail("expect:fail", ExpectedSnippetExecutionResult::Failure)]
    fn parse_expect(#[case] input: &str, #[case] expected: ExpectedSnippetExecutionResult) {
        let attributes = parse_attributes(&format!("bash +{input}"));
        assert_eq!(attributes.expected_execution_result, expected);
    }
}
