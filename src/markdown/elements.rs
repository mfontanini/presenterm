use unicode_width::UnicodeWidthStr;

use crate::style::TextStyle;
use std::iter;

/// A markdown element.
///
/// This represents each of the supported markdown elements. The structure here differs a bit from
/// the spec, mostly in how inlines are handled, to simplify its processing.
#[derive(Clone, Debug)]
pub enum MarkdownElement {
    /// The front matter that optionally shows up at the beginning of the file.
    FrontMatter(String),

    /// A setex heading.
    SetexHeading { text: Text },

    /// A normal heading.
    Heading { level: u8, text: Text },

    /// A paragraph, composed of text and line breaks.
    Paragraph(Vec<ParagraphElement>),

    /// An image.
    Image(String),

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
    Comment(String),

    /// A quote.
    BlockQuote(Vec<String>),
}

/// The components that make up a paragraph.
///
/// This does not map one-to-one with the commonmark spec and only handles text (including its
/// style) and line breaks. Any other inlines that could show up on a paragraph, such as images,
/// are a [MarkdownElement] on their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParagraphElement {
    /// A block of text.
    Text(Text),

    /// A line break.
    LineBreak,
}

/// A piece of styled text.
///
/// Text is represented as a series of chunks, each with their own formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    /// The chunks that make up this text.
    pub chunks: Vec<StyledText>,
}

impl Text {
    /// Get the total width for this text.
    pub fn width(&self) -> usize {
        self.chunks.iter().map(|text| text.text.width()).sum()
    }

    /// Applies the given style to this text.
    pub fn apply_style(&mut self, style: &TextStyle) {
        for text in &mut self.chunks {
            text.style.merge(style);
        }
    }
}

impl<T: Into<StyledText>> From<T> for Text {
    fn from(text: T) -> Self {
        Self { chunks: vec![text.into()] }
    }
}

/// A styled piece of text.
///
/// This is the most granular text representation: a `String` and a style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledText {
    pub text: String,
    pub style: TextStyle,
}

impl StyledText {
    /// Construct a new styled text.
    pub fn new<S: Into<String>>(text: S, style: TextStyle) -> Self {
        Self { text: text.into(), style }
    }
}

impl From<String> for StyledText {
    fn from(text: String) -> Self {
        Self { text, style: TextStyle::default() }
    }
}

impl From<&str> for StyledText {
    fn from(text: &str) -> Self {
        Self { text: text.into(), style: TextStyle::default() }
    }
}

/// A list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    /// The depth of this item.
    ///
    /// This increases by one for every nested list level.
    pub depth: u8,

    /// The contents of this list item.
    pub contents: Text,

    /// The type of list item.
    pub item_type: ListItemType,
}

/// The type of a list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListItemType {
    /// A list item for an unordered list.
    Unordered,

    /// A list item for an ordered list that uses parenthesis after the list item number.
    OrderedParens(u16),

    /// A list item for an ordered list that uses a period after the list item number.
    OrderedPeriod(u16),
}

/// A piece of code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    /// The code itself.
    pub contents: String,

    /// The programming language this code is written in.
    pub language: ProgrammingLanguage,
}

/// A programming language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgrammingLanguage {
    Asp,
    Bash,
    BatchFile,
    C,
    CSharp,
    Clojure,
    Cpp,
    Css,
    DLang,
    Erlang,
    Go,
    Haskell,
    Html,
    Java,
    JavaScript,
    Json,
    Latex,
    Lua,
    Makefile,
    Markdown,
    OCaml,
    Perl,
    Php,
    Python,
    R,
    Rust,
    Scala,
    Shell,
    Sql,
    TypeScript,
    Unknown,
    Xml,
    Yaml,
}

/// A table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// The table's header.
    pub header: TableRow,

    /// All of the rows in this table, excluding the header.
    pub rows: Vec<TableRow>,
}

impl Table {
    /// gets the number of columns in this table.
    pub fn columns(&self) -> usize {
        self.header.0.len()
    }

    /// Iterates all the text entries in a column.
    ///
    /// This includes the header.
    pub fn iter_column(&self, column: usize) -> impl Iterator<Item = &Text> {
        let header_element = &self.header.0[column];
        let row_elements = self.rows.iter().map(move |row| &row.0[column]);
        iter::once(header_element).chain(row_elements)
    }
}

/// A table row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow(pub Vec<Text>);
