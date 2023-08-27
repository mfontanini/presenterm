#[derive(Debug)]
pub enum Element {
    Heading { level: u8, text: Text },
    Paragraph { text: Text },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Text {
    pub chunks: Vec<TextChunk>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TextChunk {
    pub text: String,
    pub format: TextFormat,
}

impl TextChunk {
    pub fn unformatted<S: Into<String>>(text: S) -> Self {
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
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
}
