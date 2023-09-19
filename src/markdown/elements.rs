use crate::style::TextStyle;
use serde::Deserialize;
use std::iter;

#[derive(Clone, Debug)]
pub enum MarkdownElement {
    PresentationMetadata(PresentationMetadata),
    SlideTitle { text: Text },
    Heading { level: u8, text: Text },
    Paragraph(Vec<ParagraphElement>),
    List(Vec<ListItem>),
    Code(Code),
    Table(Table),
    ThematicBreak,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParagraphElement {
    Text(Text),
    Image { url: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub chunks: Vec<TextChunk>,
}

impl Text {
    pub fn single(text: StyledText) -> Self {
        Self { chunks: vec![TextChunk::Styled(text)] }
    }

    pub fn line_len(&self) -> usize {
        let mut total = 0;
        for chunk in &self.chunks {
            // TODO: what about line breaks?
            if let TextChunk::Styled(text) = &chunk {
                total += text.text.len();
            }
        }
        total
    }

    pub fn apply_style(&mut self, style: &TextStyle) {
        for chunk in &mut self.chunks {
            if let TextChunk::Styled(text) = chunk {
                text.style.merge(style);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextChunk {
    Styled(StyledText),
    LineBreak,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledText {
    pub text: String,
    pub style: TextStyle,
}

impl StyledText {
    pub fn plain<S: Into<String>>(text: S) -> Self {
        Self { text: text.into(), style: TextStyle::default() }
    }

    pub fn styled<S: Into<String>>(text: S, style: TextStyle) -> Self {
        Self { text: text.into(), style }
    }
}

impl From<String> for StyledText {
    fn from(text: String) -> Self {
        Self::plain(text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub depth: u8,
    pub contents: Text,
    pub item_type: ListItemType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListItemType {
    Unordered,
    OrderedParens(u16),
    OrderedPeriod(u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    pub contents: String,
    pub language: CodeLanguage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeLanguage {
    Rust,
    Go,
    C,
    Cpp,
    Python,
    Typescript,
    Javascript,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PresentationMetadata {
    pub title: String,

    #[serde(default)]
    pub sub_title: Option<String>,

    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub header: TableRow,
    pub rows: Vec<TableRow>,
}

impl Table {
    pub fn columns(&self) -> usize {
        self.header.0.len()
    }

    pub fn iter_column(&self, column: usize) -> impl Iterator<Item = &Text> {
        let header_element = &self.header.0[column];
        let row_elements = self.rows.iter().map(move |row| &row.0[column]);
        iter::once(header_element).chain(row_elements)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow(pub Vec<Text>);
