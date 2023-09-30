use crate::style::TextStyle;
use std::iter;

#[derive(Clone, Debug)]
pub enum MarkdownElement {
    FrontMatter(String),
    SetexHeading { text: Text },
    Heading { level: u8, text: Text },
    Paragraph(Vec<ParagraphElement>),
    List(Vec<ListItem>),
    Code(Code),
    Table(Table),
    ThematicBreak,
    Comment(String),
    BlockQuote(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParagraphElement {
    Text(Text),
    Image { url: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub chunks: Vec<StyledText>,
}

impl Text {
    pub fn single(text: StyledText) -> Self {
        Self { chunks: vec![text] }
    }

    pub fn line_len(&self) -> usize {
        self.chunks.iter().map(|text| text.text.len()).sum()
    }

    pub fn apply_style(&mut self, style: &TextStyle) {
        for text in &mut self.chunks {
            text.style.merge(style);
        }
    }
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

impl From<&str> for StyledText {
    fn from(text: &str) -> Self {
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
