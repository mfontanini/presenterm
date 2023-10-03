use crate::theme::Colors;
use crossterm::style::Stylize;

/// The style of a piece of text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextStyle {
    flags: u8,
    pub colors: Colors,
}

impl TextStyle {
    /// Add bold to this style.
    pub fn bold(mut self) -> Self {
        self.flags |= TextFormatFlags::Bold as u8;
        self
    }

    /// Add italics to this style.
    pub fn italics(mut self) -> Self {
        self.flags |= TextFormatFlags::Italics as u8;
        self
    }

    /// Indicate this text is a piece of inline code.
    pub fn code(mut self) -> Self {
        self.flags |= TextFormatFlags::Code as u8;
        self
    }

    /// Add strikethrough to this style.
    pub fn strikethrough(mut self) -> Self {
        self.flags |= TextFormatFlags::Strikethrough as u8;
        self
    }

    /// Indicate this is a link.
    pub fn link(mut self) -> Self {
        self.flags |= TextFormatFlags::Link as u8;
        self
    }

    /// Set the colors for this text style.
    pub fn colors(mut self, colors: Colors) -> Self {
        self.colors = colors;
        self
    }

    /// Check whether this text style is bold.
    pub fn is_bold(&self) -> bool {
        self.flags & TextFormatFlags::Bold as u8 != 0
    }

    /// Check whether this text style has italics.
    pub fn is_italics(&self) -> bool {
        self.flags & TextFormatFlags::Italics as u8 != 0
    }

    /// Check whether this text is code.
    pub fn is_code(&self) -> bool {
        self.flags & TextFormatFlags::Code as u8 != 0
    }

    /// Check whether this text style is strikethrough.
    pub fn is_strikethrough(&self) -> bool {
        self.flags & TextFormatFlags::Strikethrough as u8 != 0
    }

    /// Check whether this text is a link.
    pub fn is_link(&self) -> bool {
        self.flags & TextFormatFlags::Link as u8 != 0
    }

    /// Merge this style with another one.
    pub fn merge(&mut self, other: &TextStyle) {
        self.flags |= other.flags;
        self.colors.background = self.colors.background.or(other.colors.background);
        self.colors.foreground = self.colors.foreground.or(other.colors.foreground);
    }

    /// Apply this style to a piece of text.
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
