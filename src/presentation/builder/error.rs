use crate::{
    code::execute::UnsupportedExecution,
    markdown::{elements::SourcePosition, parse::ParseError, text_style::UndefinedPaletteColorError},
    presentation::builder::{comment::CommandParseError, images::ImageAttributeError, sources::MarkdownSourceError},
    terminal::image::printer::RegisterImageError,
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

    #[error("invalid presentation metadata: {0}")]
    InvalidMetadata(String),

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

    #[error("invalid presentation title: {0}")]
    PresentationTitle(String),

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

    #[error("need to enter layout column explicitly using `column` command")]
    NotInsideColumn,
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
}

#[derive(Debug)]
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
