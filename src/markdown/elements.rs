use crate::theme::{ColorPalette, raw::RawColor};

use super::text_style::{Color, TextStyle, UndefinedPaletteColorError};
use comrak::nodes::AlertType;
use std::{fmt, iter, path::PathBuf, str::FromStr};
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
    SetexHeading { text: Line<RawColor> },

    /// A normal heading.
    Heading { level: u8, text: Line<RawColor> },

    /// A paragraph composed by a list of lines.
    Paragraph(Vec<Line<RawColor>>),

    /// An image.
    Image { path: PathBuf, title: String, source_position: SourcePosition },

    /// A list.
    ///
    /// All contiguous list items are merged into a single one, regardless of levels of nesting.
    List(Vec<ListItem>),

    /// A code snippet.
    Snippet {
        /// The information line that specifies this code's language, attributes, etc.
        info: String,

        /// The code in this snippet.
        code: String,

        /// The position in the source file this snippet came from.
        source_position: SourcePosition,
    },

    /// A table.
    Table(Table),

    /// A thematic break.
    ThematicBreak,

    /// An HTML comment.
    Comment { comment: String, source_position: SourcePosition },

    /// A block quote containing a list of lines.
    BlockQuote(Vec<Line<RawColor>>),

    /// An alert.
    Alert {
        /// The alert's type.
        alert_type: AlertType,

        /// The optional title.
        title: Option<String>,

        /// The content lines in this alert.
        lines: Vec<Line<RawColor>>,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SourcePosition {
    pub(crate) start: LineColumn,
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start.line, self.start.column)
    }
}

impl From<comrak::nodes::Sourcepos> for SourcePosition {
    fn from(position: comrak::nodes::Sourcepos) -> Self {
        Self { start: position.start.into() }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LineColumn {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl From<comrak::nodes::LineColumn> for LineColumn {
    fn from(position: comrak::nodes::LineColumn) -> Self {
        Self { line: position.line, column: position.column }
    }
}

/// A text line.
///
/// Text is represented as a series of chunks, each with their own formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Line<C = Color>(pub(crate) Vec<Text<C>>);

impl<C> Default for Line<C> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<C> Line<C> {
    /// Get the total width for this text.
    pub(crate) fn width(&self) -> usize {
        self.0.iter().map(|text| text.content.width()).sum()
    }
}

impl Line<Color> {
    /// Applies the given style to this text.
    pub(crate) fn apply_style(&mut self, style: &TextStyle) {
        for text in &mut self.0 {
            text.style.merge(style);
        }
    }
}

impl Line<RawColor> {
    /// Resolve the colors in this line.
    pub(crate) fn resolve(self, palette: &ColorPalette) -> Result<Line<Color>, UndefinedPaletteColorError> {
        let mut output = Vec::with_capacity(self.0.len());
        for text in self.0 {
            let style = text.style.resolve(palette)?;
            output.push(Text::new(text.content, style));
        }
        Ok(Line(output))
    }
}

impl<C, T: Into<Text<C>>> From<T> for Line<C> {
    fn from(text: T) -> Self {
        Self(vec![text.into()])
    }
}

/// A styled piece of text.
///
/// This is the most granular text representation: a `String` and a style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Text<C = Color> {
    pub(crate) content: String,
    pub(crate) style: TextStyle<C>,
}

impl<C> Text<C> {
    /// Construct a new styled text.
    pub(crate) fn new<S: Into<String>>(content: S, style: TextStyle<C>) -> Self {
        Self { content: content.into(), style }
    }
}

impl<C> From<String> for Text<C> {
    fn from(text: String) -> Self {
        Self { content: text, style: TextStyle::default() }
    }
}

impl<C> From<&str> for Text<C> {
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
    pub(crate) contents: Line<RawColor>,

    /// The type of list item.
    pub(crate) item_type: ListItemType,
}

/// The type of a list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListItemType {
    /// A list item for an unordered list.
    Unordered,

    /// A list item for an ordered list that uses parenthesis after the list item number.
    OrderedParens(usize),

    /// A list item for an ordered list that uses a period after the list item number.
    OrderedPeriod(usize),
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
    pub(crate) fn iter_column(&self, column: usize) -> impl Iterator<Item = &Line<RawColor>> {
        let header_element = &self.header.0[column];
        let row_elements = self.rows.iter().map(move |row| &row.0[column]);
        iter::once(header_element).chain(row_elements)
    }
}

/// A table row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TableRow(pub(crate) Vec<Line<RawColor>>);

/// A percentage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Percent(pub(crate) u8);

impl Percent {
    pub(crate) fn as_ratio(&self) -> f64 {
        self.0 as f64 / 100.0
    }
}

impl FromStr for Percent {
    type Err = PercentParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (prefix, suffix) = input.split_once('%').ok_or(PercentParseError::Unit)?;
        let value: u8 = prefix.parse().map_err(|_| PercentParseError::Value)?;
        if !(1..=100).contains(&value) {
            return Err(PercentParseError::Value);
        }
        if !suffix.is_empty() {
            return Err(PercentParseError::Trailer(suffix.into()));
        }
        Ok(Percent(value))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PercentParseError {
    #[error("value must be a number between 1-100")]
    Value,

    #[error("no unit provided")]
    Unit,

    #[error("unexpected: '{0}'")]
    Trailer(String),
}
