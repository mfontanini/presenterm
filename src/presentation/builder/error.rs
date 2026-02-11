use crate::{
    code::execute::UnsupportedExecution,
    markdown::{
        elements::SourcePosition,
        parse::ParseError,
        text_style::{Color, TextStyle, UndefinedPaletteColorError},
    },
    presentation::builder::{comment::CommandParseError, images::ImageAttributeError, sources::MarkdownSourceError},
    terminal::{capabilities::TerminalCapabilities, image::printer::RegisterImageError},
    theme::{ProcessingThemeError, registry::LoadThemeError},
    third_party::ThirdPartyRenderError,
    ui::footer::InvalidFooterTemplateError,
};
use std::{
    fmt,
    io::{self},
    path::PathBuf,
};

/// An error when building a presentation.
#[derive(thiserror::Error, Debug)]
pub(crate) enum BuildError {
    #[error("failed to read presentation file {0:?}: {1:?}")]
    ReadPresentation(PathBuf, io::Error),

    #[error("failed to register image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("invalid theme: {0}")]
    InvalidTheme(#[from] LoadThemeError),

    #[error("invalid code highlighter theme: '{0}'")]
    InvalidCodeTheme(String),

    #[error("third party render failed: {0}")]
    ThirdPartyRender(#[from] ThirdPartyRenderError),

    #[error(transparent)]
    UnsupportedExecution(#[from] UnsupportedExecution),

    #[error(transparent)]
    UndefinedPaletteColor(#[from] UndefinedPaletteColorError),

    #[error("processing theme: {0}")]
    ThemeProcessing(#[from] ProcessingThemeError),

    #[error("invalid footer: {0}")]
    InvalidFooter(#[from] InvalidFooterTemplateError),

    #[error(
        "invalid markdown at {display_path}:{line}:{column}:\n\n{context}",
        display_path = .path.display(),
        line = .error.sourcepos.start.line,
        column = .error.sourcepos.start.column,
    )]
    Parse { path: PathBuf, error: ParseError, context: String },

    #[error("cannot process presentation file: {0}")]
    EnterRoot(MarkdownSourceError),

    #[error(
        "error at {display_path}:{line}:{column}:\n\n{context}",
        display_path = .path.display(),
        line = .source_position.start.line,
        column = .source_position.start.column,
    )]
    InvalidPresentation { path: PathBuf, source_position: SourcePosition, context: String },

    #[error("error in frontmatter:\n\n{0}")]
    InvalidFrontmatter(String),

    #[error("need to enter layout column explicitly using `column` command\n\n{0}")]
    NotInsideColumn(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidPresentation {
    #[error("could not load image '{path}': {error}")]
    LoadImage { path: PathBuf, error: String },

    #[error("invalid image attribute: {0}")]
    ParseImageAttribute(#[from] ImageAttributeError),

    #[error("invalid snippet: {0}")]
    Snippet(String),

    #[error("invalid command: {0}")]
    CommandParse(#[from] CommandParseError),

    #[error("invalid markdown in imported file {path:?}: {error}")]
    ParseInclude { path: PathBuf, error: ParseError },

    #[error("could not read included markdown file {path:?}: {error}")]
    IncludeMarkdown { path: PathBuf, error: io::Error },

    #[error("included markdown files cannot contain a front matter")]
    IncludeFrontMatter,

    #[error("cannot include markdown file at {path}: {error}")]
    Import { path: PathBuf, error: MarkdownSourceError },

    #[error("can't enter layout: no layout defined")]
    NoLayout,

    #[error("can't enter layout column: already in it")]
    AlreadyInColumn,

    #[error("can't enter layout column: column index too large")]
    ColumnIndexTooLarge,

    #[error("invalid layout: {0}")]
    InvalidLayout(&'static str),

    #[error("font sizes must be >= 1 and <= 7")]
    InvalidFontSize,

    #[error("snippet id '{0}' not defined")]
    UndefinedSnippetId(String),

    #[error("snippet identifiers can only be used in +exec blocks")]
    SnippetIdNonExec,

    #[error("snippet id '{0}' already exists")]
    SnippetAlreadyExists(String),

    #[error("invalid color: {0}")]
    InvalidColor(#[from] UndefinedPaletteColorError),
}

#[derive(Clone, Debug)]
pub(crate) struct FileSourcePosition {
    pub(crate) source_position: SourcePosition,
    pub(crate) file: PathBuf,
}

impl fmt::Display for FileSourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.display();
        let pos = &self.source_position;
        write!(f, "{file}:{pos}")
    }
}

pub(super) trait FormatError {
    fn format_error(self) -> String;
}

impl FormatError for String {
    fn format_error(self) -> String {
        TextStyle::default().fg_color(Color::Red).apply(&self, &Default::default()).to_string()
    }
}

#[derive(Default)]
pub(super) struct ErrorContextBuilder<'a> {
    line: Option<usize>,
    column: Option<usize>,
    source_line: &'a str,
    error: &'a str,
    prefix_style: TextStyle,
    error_style: TextStyle,
}

impl<'a> ErrorContextBuilder<'a> {
    pub(super) fn new(source_line: &'a str, error: &'a str) -> Self {
        Self {
            line: None,
            column: None,
            source_line,
            error,
            prefix_style: TextStyle::default().fg_color(Color::Blue),
            error_style: TextStyle::default().fg_color(Color::Red),
        }
    }

    pub(super) fn position(mut self, position: SourcePosition) -> Self {
        self.line = Some(position.start.line);
        self.column = Some(position.start.column);
        self
    }

    pub(super) fn column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub(super) fn build(self) -> String {
        let Self { line, column, source_line, error, prefix_style, error_style } = self;
        let (error_line_prefix, empty_line, source_line) = match line {
            Some(line) => {
                let line_number = line.to_string();
                let empty_prefix = " ".repeat(line_number.len());
                let error_line_prefix = format!("{line_number} | ");
                let empty_line = format!("{empty_prefix} | ");
                let source_line = source_line.lines().nth(line.saturating_sub(1)).unwrap_or_default();
                (error_line_prefix, empty_line, source_line)
            }
            None => {
                let prefix = " | ".to_string();
                (prefix.clone(), prefix, source_line)
            }
        };
        let column = column.map(|c| c.saturating_sub(1)).unwrap_or_default();
        let capabilities = TerminalCapabilities::default();
        let empty_line = prefix_style.apply(&empty_line, &capabilities).to_string();
        let mut output = empty_line.clone();
        output.push('\n');
        let prefix = prefix_style.apply(&error_line_prefix, &capabilities).to_string();
        output.push_str(&format!("{prefix}{source_line}\n"));

        let indicator = format!("{}^ {error}", " ".repeat(column));
        let indicator = error_style.apply(&indicator, &capabilities).to_string();
        let indicator_line = format!("{empty_line}{indicator}");
        output.push_str(&indicator_line);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::elements::LineColumn;

    trait ErrorContextBuilderExt {
        fn into_lines(self) -> Vec<String>;
    }

    impl ErrorContextBuilderExt for ErrorContextBuilder<'_> {
        fn into_lines(self) -> Vec<String> {
            let error = self.build();
            error.lines().map(ToString::to_string).collect()
        }
    }

    fn make_builder<'a>(source_line: &'a str, error: &'a str) -> ErrorContextBuilder<'a> {
        let mut builder = ErrorContextBuilder::new(source_line, error);
        builder.prefix_style = Default::default();
        builder.error_style = Default::default();
        builder
    }

    #[test]
    fn position() {
        let lines = make_builder("foo\nbear\ntar", "'a' not allowed")
            .position(SourcePosition { start: LineColumn { line: 2, column: 3 } })
            .into_lines();
        let expected = &[
            //
            "  | ",
            "2 | bear",
            "  |   ^ 'a' not allowed",
        ];
        assert_eq!(&lines, expected);
    }

    #[test]
    fn no_position() {
        let lines = make_builder("bear", "'b' not allowed").into_lines();
        let expected = &[
            //
            " | ",
            " | bear",
            " | ^ 'b' not allowed",
        ];
        assert_eq!(&lines, expected);
    }

    #[test]
    fn column() {
        let lines = make_builder("bear", "'e' not allowed").column(2).into_lines();
        let expected = &[
            //
            " | ",
            " | bear",
            " |  ^ 'e' not allowed",
        ];
        assert_eq!(&lines, expected);
    }
}
