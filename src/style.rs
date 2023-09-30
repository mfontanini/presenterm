use crate::theme::Colors;
use crossterm::style::Stylize;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextStyle {
    flags: u8,
    pub colors: Colors,
}

impl TextStyle {
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

    pub fn link(mut self) -> Self {
        self.flags |= TextFormatFlags::Link as u8;
        self
    }

    pub fn colors(mut self, colors: Colors) -> Self {
        self.colors = colors;
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

    pub fn is_link(&self) -> bool {
        self.flags & TextFormatFlags::Link as u8 != 0
    }

    pub fn merge(&mut self, other: &TextStyle) {
        self.flags |= other.flags;
        self.colors.background = self.colors.background.or(other.colors.background);
        self.colors.foreground = self.colors.foreground.or(other.colors.foreground);
    }

    pub fn apply<T: Into<String>>(&self, text: T) -> <String as Stylize>::Styled {
        let text: String = text.into();
        let mut styled = text.stylize();
        if self.is_bold() {
            styled = styled.bold();
        }
        if self.is_italics() {
            styled = styled.italic();
        }
        if self.is_strikethrough() {
            styled = styled.crossed_out();
        }
        if self.is_link() {
            styled = styled.italic().underlined();
        }
        if let Some(color) = self.colors.background {
            styled = styled.on(color);
        }
        if let Some(color) = self.colors.foreground {
            styled = styled.with(color);
        }
        styled
    }
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
    Code = 4,
    Strikethrough = 8,
    Link = 16,
}
