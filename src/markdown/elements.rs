use std::iter;

use serde::Deserialize;

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
    pub fn single(text: FormattedText) -> Self {
        Self { chunks: vec![TextChunk::Formatted(text)] }
    }

    pub fn line_len(&self) -> usize {
        let mut total = 0;
        for chunk in &self.chunks {
            // TODO: what about line breaks?
            if let TextChunk::Formatted(text) = &chunk {
                total += text.text.len();
            }
        }
        total
    }

    pub fn apply_format(&mut self, format: &TextFormat) {
        for chunk in &mut self.chunks {
            if let TextChunk::Formatted(text) = chunk {
                text.format.merge(format);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextChunk {
    Formatted(FormattedText),
    LineBreak,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormattedText {
    pub text: String,
    pub format: TextFormat,
}

impl FormattedText {
    pub fn plain<S: Into<String>>(text: S) -> Self {
        Self { text: text.into(), format: TextFormat::default() }
    }

    pub fn formatted<S: Into<String>>(text: S, format: TextFormat) -> Self {
        Self { text: text.into(), format }
    }
}

impl From<String> for FormattedText {
    fn from(text: String) -> Self {
        Self::plain(text)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextFormat(u8);

impl TextFormat {
    pub fn add_bold(mut self) -> Self {
        self.0 |= TextFormatFlags::Bold as u8;
        self
    }

    pub fn add_italics(mut self) -> Self {
        self.0 |= TextFormatFlags::Italics as u8;
        self
    }

    pub fn add_code(mut self) -> Self {
        self.0 |= TextFormatFlags::Code as u8;
        self
    }

    pub fn add_strikethrough(mut self) -> Self {
        self.0 |= TextFormatFlags::Strikethrough as u8;
        self
    }

    pub fn has_bold(&self) -> bool {
        self.0 & TextFormatFlags::Bold as u8 != 0
    }

    pub fn has_italics(&self) -> bool {
        self.0 & TextFormatFlags::Italics as u8 != 0
    }

    pub fn has_code(&self) -> bool {
        self.0 & TextFormatFlags::Code as u8 != 0
    }

    pub fn has_strikethrough(&self) -> bool {
        self.0 & TextFormatFlags::Strikethrough as u8 != 0
    }

    pub fn merge(&mut self, other: &TextFormat) {
        self.0 |= other.0
    }
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
    Code = 4,
    Strikethrough = 8,
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
