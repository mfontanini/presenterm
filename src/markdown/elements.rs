use crate::style::TextStyle;
use std::{iter, ops::Range, path::PathBuf};
use strum::EnumIter;
use unicode_width::UnicodeWidthStr;

/// A markdown element.
///
/// This represents each of the supported markdown elements. The structure here differs a bit from
/// the spec, mostly in how inlines are handled, to simplify its processing.
#[derive(Clone, Debug)]
pub(crate) enum MarkdownElement {
    /// The front matter that optionally shows up at the beginning of the file.
    FrontMatter(String),

    /// A setex heading.
    SetexHeading { text: TextBlock },

    /// A normal heading.
    Heading { level: u8, text: TextBlock },

    /// A paragraph, composed of text and line breaks.
    Paragraph(Vec<ParagraphElement>),

    /// An image.
    Image { path: PathBuf },

    /// A list.
    ///
    /// All contiguous list items are merged into a single one, regardless of levels of nesting.
    List(Vec<ListItem>),

    /// A block of code.
    Code(Code),

    /// A table.
    Table(Table),

    /// A thematic break.
    ThematicBreak,

    /// An HTML comment.
    Comment { comment: String, source_position: SourcePosition },

    /// A quote.
    BlockQuote(Vec<String>),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SourcePosition {
    pub(crate) start: LineColumn,
}

impl SourcePosition {
    pub(crate) fn offset_lines(&self, offset: usize) -> SourcePosition {
        let mut output = self.clone();
        output.start.line += offset;
        output
    }
}

impl From<comrak::nodes::Sourcepos> for SourcePosition {
    fn from(position: comrak::nodes::Sourcepos) -> Self {
        Self { start: position.start.into() }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LineColumn {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl From<comrak::nodes::LineColumn> for LineColumn {
    fn from(position: comrak::nodes::LineColumn) -> Self {
        Self { line: position.line, column: position.column }
    }
}

/// The components that make up a paragraph.
///
/// This does not map one-to-one with the commonmark spec and only handles text (including its
/// style) and line breaks. Any other inlines that could show up on a paragraph, such as images,
/// are a [MarkdownElement] on their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParagraphElement {
    /// A block of text.
    Text(TextBlock),

    /// A line break.
    LineBreak,
}

/// A block of text.
///
/// Text is represented as a series of chunks, each with their own formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextBlock(pub(crate) Vec<Text>);

impl TextBlock {
    /// Get the total width for this text.
    pub(crate) fn width(&self) -> usize {
        self.0.iter().map(|text| text.content.width()).sum()
    }

    /// Applies the given style to this text.
    pub(crate) fn apply_style(&mut self, style: &TextStyle) {
        for text in &mut self.0 {
            text.style.merge(style);
        }
    }
}

impl<T: Into<Text>> From<T> for TextBlock {
    fn from(text: T) -> Self {
        Self(vec![text.into()])
    }
}

/// A styled piece of text.
///
/// This is the most granular text representation: a `String` and a style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Text {
    pub(crate) content: String,
    pub(crate) style: TextStyle,
}

impl Text {
    /// Construct a new styled text.
    pub(crate) fn new<S: Into<String>>(content: S, style: TextStyle) -> Self {
        Self { content: content.into(), style }
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self { content: text, style: TextStyle::default() }
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self { content: text.into(), style: TextStyle::default() }
    }
}

/// A list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListItem {
    /// The depth of this item.
    ///
    /// This increases by one for every nested list level.
    pub(crate) depth: u8,

    /// The contents of this list item.
    pub(crate) contents: TextBlock,

    /// The type of list item.
    pub(crate) item_type: ListItemType,
}

/// The type of a list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListItemType {
    /// A list item for an unordered list.
    Unordered,

    /// A list item for an ordered list that uses parenthesis after the list item number.
    OrderedParens,

    /// A list item for an ordered list that uses a period after the list item number.
    OrderedPeriod,
}

/// A piece of code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Code {
    /// The code itself.
    pub(crate) contents: String,

    /// The programming language this code is written in.
    pub(crate) language: CodeLanguage,

    /// The attributes used for this code.
    pub(crate) attributes: CodeAttributes,
}

/// The language of a piece of code.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter)]
pub(crate) enum CodeLanguage {
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
    DLang,
    Diff,
    Docker,
    Dotenv,
    Elixir,
    Elm,
    Erlang,
    Go,
    Haskell,
    Html,
    Java,
    JavaScript,
    Json,
    Kotlin,
    Latex,
    Lua,
    Makefile,
    Markdown,
    Nix,
    OCaml,
    Perl,
    Php,
    Protobuf,
    Puppet,
    Python,
    R,
    Ruby,
    Rust,
    Scala,
    Shell(String),
    Sql,
    Swift,
    Svelte,
    Terraform,
    TypeScript,
    Typst,
    Unknown,
    Xml,
    Yaml,
    Vue,
    Zig,
}

impl CodeLanguage {
    pub(crate) fn supports_auto_render(&self) -> bool {
        matches!(self, Self::Latex | Self::Typst)
    }
}

/// Attributes for code blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CodeAttributes {
    /// Whether the code block is marked as executable.
    pub(crate) execute: bool,

    /// Whether a code block is marked to be auto rendered.
    ///
    /// An auto rendered piece of code is transformed during parsing, leading to some visual
    /// representation of it being shown rather than the original code.
    pub(crate) auto_render: bool,

    /// Whether the code block should show line numbers.
    pub(crate) line_numbers: bool,

    /// The groups of lines to highlight.
    pub(crate) highlight_groups: Vec<HighlightGroup>,
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

/// A table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Table {
    /// The table's header.
    pub(crate) header: TableRow,

    /// All of the rows in this table, excluding the header.
    pub(crate) rows: Vec<TableRow>,
}

impl Table {
    /// gets the number of columns in this table.
    pub(crate) fn columns(&self) -> usize {
        self.header.0.len()
    }

    /// Iterates all the text entries in a column.
    ///
    /// This includes the header.
    pub(crate) fn iter_column(&self, column: usize) -> impl Iterator<Item = &TextBlock> {
        let header_element = &self.header.0[column];
        let row_elements = self.rows.iter().map(move |row| &row.0[column]);
        iter::once(header_element).chain(row_elements)
    }
}

/// A table row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TableRow(pub(crate) Vec<TextBlock>);
