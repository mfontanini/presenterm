#[derive(Clone, Debug)]
pub enum Element {
    Heading { level: u8, text: Text },
    Paragraph(Text),
    List(Vec<ListItem>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub chunks: Vec<TextChunk>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextChunk {
    Formatted(FormattedText),
    Image { title: String, url: String },
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

    pub fn has_bold(&self) -> bool {
        self.0 & TextFormatFlags::Bold as u8 != 0
    }

    pub fn has_italics(&self) -> bool {
        self.0 & TextFormatFlags::Italics as u8 != 0
    }
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
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
