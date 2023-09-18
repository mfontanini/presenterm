use crate::theme::Colors;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextFormat {
    flags: u8,
    pub colors: Colors,
}

impl TextFormat {
    pub fn bold(mut self) -> Self {
        self.flags |= TextFormatFlags::Bold as u8;
        self
    }

    pub fn italics(mut self) -> Self {
        self.flags |= TextFormatFlags::Italics as u8;
        self
    }

    pub fn code(mut self) -> Self {
        self.flags |= TextFormatFlags::Code as u8;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.flags |= TextFormatFlags::Strikethrough as u8;
        self
    }

    pub fn is_bold(&self) -> bool {
        self.flags & TextFormatFlags::Bold as u8 != 0
    }

    pub fn is_italics(&self) -> bool {
        self.flags & TextFormatFlags::Italics as u8 != 0
    }

    pub fn is_code(&self) -> bool {
        self.flags & TextFormatFlags::Code as u8 != 0
    }

    pub fn is_strikethrough(&self) -> bool {
        self.flags & TextFormatFlags::Strikethrough as u8 != 0
    }

    pub fn merge(&mut self, other: &TextFormat) {
        self.flags |= other.flags
    }
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
    Code = 4,
    Strikethrough = 8,
}
