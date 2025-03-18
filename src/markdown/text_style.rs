use crate::theme::{ColorPalette, raw::RawColor};
use crossterm::style::{StyledContent, Stylize};
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// The style of a piece of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextStyle<C = Color> {
    flags: u8,
    pub(crate) colors: Colors<C>,
    pub(crate) size: u8,
}

impl<C> Default for TextStyle<C> {
    fn default() -> Self {
        Self { flags: Default::default(), colors: Default::default(), size: 1 }
    }
}

impl<C> TextStyle<C>
where
    C: Clone,
{
    pub(crate) fn colored(colors: Colors<C>) -> Self {
        Self { colors, ..Default::default() }
    }

    pub(crate) fn size(mut self, size: u8) -> Self {
        self.size = size.min(16);
        self
    }

    /// Add bold to this style.
    pub(crate) fn bold(self) -> Self {
        self.add_flag(TextFormatFlags::Bold)
    }

    /// Add italics to this style.
    pub(crate) fn italics(self) -> Self {
        self.add_flag(TextFormatFlags::Italics)
    }

    /// Indicate this text is a piece of inline code.
    pub(crate) fn code(self) -> Self {
        self.add_flag(TextFormatFlags::Code)
    }

    /// Add strikethrough to this style.
    pub(crate) fn strikethrough(self) -> Self {
        self.add_flag(TextFormatFlags::Strikethrough)
    }

    /// Add underline to this style.
    pub(crate) fn underlined(self) -> Self {
        self.add_flag(TextFormatFlags::Underlined)
    }

    /// Indicate this is a link label.
    pub(crate) fn link_label(self) -> Self {
        self.bold()
    }

    /// Indicate this is a link title.
    pub(crate) fn link_title(self) -> Self {
        self.italics()
    }

    /// Indicate this is a link url.
    pub(crate) fn link_url(self) -> Self {
        self.italics().underlined()
    }

    /// Set the background color for this text style.
    pub(crate) fn bg_color<U: Into<C>>(mut self, color: U) -> Self {
        self.colors.background = Some(color.into());
        self
    }

    /// Set the foreground color for this text style.
    pub(crate) fn fg_color<U: Into<C>>(mut self, color: U) -> Self {
        self.colors.foreground = Some(color.into());
        self
    }

    /// Set the colors on this style.
    pub(crate) fn colors(mut self, colors: Colors<C>) -> Self {
        self.colors = colors;
        self
    }

    /// Check whether this text style is bold.
    pub(crate) fn is_bold(&self) -> bool {
        self.has_flag(TextFormatFlags::Bold)
    }

    /// Check whether this text style has italics.
    pub(crate) fn is_italics(&self) -> bool {
        self.has_flag(TextFormatFlags::Italics)
    }

    /// Check whether this text is code.
    pub(crate) fn is_code(&self) -> bool {
        self.has_flag(TextFormatFlags::Code)
    }

    /// Check whether this text style is strikethrough.
    pub(crate) fn is_strikethrough(&self) -> bool {
        self.has_flag(TextFormatFlags::Strikethrough)
    }

    /// Check whether this text style is underlined.
    pub(crate) fn is_underlined(&self) -> bool {
        self.has_flag(TextFormatFlags::Underlined)
    }

    /// Merge this style with another one.
    pub(crate) fn merge(&mut self, other: &TextStyle<C>) {
        self.flags |= other.flags;
        self.size = self.size.max(other.size);
        self.colors.background = self.colors.background.clone().or(other.colors.background.clone());
        self.colors.foreground = self.colors.foreground.clone().or(other.colors.foreground.clone());
    }

    /// Return a new style merged with the one passed in.
    pub(crate) fn merged(mut self, other: &TextStyle<C>) -> Self {
        self.merge(other);
        self
    }

    fn add_flag(mut self, flag: TextFormatFlags) -> Self {
        self.flags |= flag as u8;
        self
    }

    fn has_flag(&self, flag: TextFormatFlags) -> bool {
        self.flags & flag as u8 != 0
    }
}

impl TextStyle<Color> {
    /// Apply this style to a piece of text.
    pub(crate) fn apply<'a>(&self, text: &'a str) -> StyledContent<impl Display + Clone + 'a> {
        let text = FontSizedStr { contents: text, font_size: self.size };
        let mut styled = StyledContent::new(Default::default(), text);
        if self.is_bold() {
            styled = styled.bold();
        }
        if self.is_italics() {
            styled = styled.italic();
        }
        if self.is_strikethrough() {
            styled = styled.crossed_out();
        }
        if self.is_underlined() {
            styled = styled.underlined();
        }
        if let Some(color) = self.colors.background {
            styled = styled.on(color.into());
        }
        if let Some(color) = self.colors.foreground {
            styled = styled.with(color.into());
        }
        styled
    }

    pub(crate) fn into_raw(self) -> TextStyle<RawColor> {
        let colors = Colors {
            background: self.colors.background.map(Into::into),
            foreground: self.colors.foreground.map(Into::into),
        };
        TextStyle { flags: self.flags, colors, size: self.size }
    }
}

impl TextStyle<RawColor> {
    pub(crate) fn resolve(&self, palette: &ColorPalette) -> Result<TextStyle, UndefinedPaletteColorError> {
        let colors = self.colors.resolve(palette)?;
        Ok(TextStyle { flags: self.flags, colors, size: self.size })
    }
}

#[derive(Clone)]
struct FontSizedStr<'a> {
    contents: &'a str,
    font_size: u8,
}

impl fmt::Display for FontSizedStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = &self.contents;
        match self.font_size {
            0 | 1 => write!(f, "{contents}"),
            size => write!(f, "\x1b]66;s={size};{contents}\x1b\\"),
        }
    }
}

#[derive(Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
    Code = 4,
    Strikethrough = 8,
    Underlined = 16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Color {
    Black,
    DarkGrey,
    Red,
    DarkRed,
    Green,
    DarkGreen,
    Yellow,
    DarkYellow,
    Blue,
    DarkBlue,
    Magenta,
    DarkMagenta,
    Cyan,
    DarkCyan,
    White,
    Grey,
    Rgb { r: u8, g: u8, b: u8 },
}

impl Color {
    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb { r, g, b }
    }

    pub(crate) fn as_rgb(&self) -> Option<(u8, u8, u8)> {
        match self {
            Self::Rgb { r, g, b } => Some((*r, *g, *b)),
            _ => None,
        }
    }

    pub(crate) fn from_ansi(color: u8) -> Option<Self> {
        let color = match color {
            30 | 40 => Color::Black,
            31 | 41 => Color::Red,
            32 | 42 => Color::Green,
            33 | 43 => Color::Yellow,
            34 | 44 => Color::Blue,
            35 | 45 => Color::Magenta,
            36 | 46 => Color::Cyan,
            37 | 47 => Color::White,
            _ => return None,
        };
        Some(color)
    }
}

impl From<Color> for crossterm::style::Color {
    fn from(value: Color) -> Self {
        use crossterm::style::Color as C;
        match value {
            Color::Black => C::Black,
            Color::DarkGrey => C::DarkGrey,
            Color::Red => C::Red,
            Color::DarkRed => C::DarkRed,
            Color::Green => C::Green,
            Color::DarkGreen => C::DarkGreen,
            Color::Yellow => C::Yellow,
            Color::DarkYellow => C::DarkYellow,
            Color::Blue => C::Blue,
            Color::DarkBlue => C::DarkBlue,
            Color::Magenta => C::Magenta,
            Color::DarkMagenta => C::DarkMagenta,
            Color::Cyan => C::Cyan,
            Color::DarkCyan => C::DarkCyan,
            Color::White => C::White,
            Color::Grey => C::Grey,
            Color::Rgb { r, g, b } => C::Rgb { r, g, b },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unresolved palette color: {0}")]
pub(crate) struct PaletteColorError(String);

#[derive(Debug, thiserror::Error)]
#[error("undefined palette color: {0}")]
pub(crate) struct UndefinedPaletteColorError(pub(crate) String);

/// Text colors.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct Colors<C = Color> {
    /// The background color.
    pub(crate) background: Option<C>,

    /// The foreground color.
    pub(crate) foreground: Option<C>,
}

impl<C> Default for Colors<C> {
    fn default() -> Self {
        Self { background: None, foreground: None }
    }
}

impl Colors<RawColor> {
    pub(crate) fn resolve(&self, palette: &ColorPalette) -> Result<Colors<Color>, UndefinedPaletteColorError> {
        let background = self.background.clone().map(|c| c.resolve(palette)).transpose()?.flatten();
        let foreground = self.foreground.clone().map(|c| c.resolve(palette)).transpose()?.flatten();
        Ok(Colors { foreground, background })
    }
}

impl From<Colors> for crossterm::style::Colors {
    fn from(value: Colors) -> Self {
        let foreground = value.foreground.map(Color::into);
        let background = value.background.map(Color::into);
        Self { foreground, background }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ParseColorError {
    #[error("invalid hex color: {0}")]
    Hex(#[from] FromHexError),
}
