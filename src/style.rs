use crossterm::style::Stylize;
use hex::{FromHex, FromHexError};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

/// The style of a piece of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextStyle {
    flags: u8,
    pub(crate) colors: Colors,
}

impl TextStyle {
    pub(crate) fn colored(colors: Colors) -> Self {
        Self { flags: Default::default(), colors }
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

    /// Set the colors for this text style.
    pub(crate) fn colors(mut self, colors: Colors) -> Self {
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
    pub(crate) fn merge(&mut self, other: &TextStyle) {
        self.flags |= other.flags;
        self.colors.background = self.colors.background.or(other.colors.background);
        self.colors.foreground = self.colors.foreground.or(other.colors.foreground);
    }

    /// Apply this style to a piece of text.
    pub(crate) fn apply<T: Into<String>>(&self, text: T) -> <String as Stylize>::Styled {
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

    /// Checks whether this style has any modifiers (bold, italics, etc).
    pub(crate) fn has_modifiers(&self) -> bool {
        self.flags != 0
    }

    fn add_flag(mut self, flag: TextFormatFlags) -> Self {
        self.flags |= flag as u8;
        self
    }

    fn has_flag(&self, flag: TextFormatFlags) -> bool {
        self.flags & flag as u8 != 0
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
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

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let output = match input {
            "black" => Self::Black,
            "white" => Self::White,
            "grey" => Self::Grey,
            "dark_grey" => Self::DarkGrey,
            "red" => Self::Red,
            "dark_red" => Self::DarkRed,
            "green" => Self::Green,
            "dark_green" => Self::DarkGreen,
            "blue" => Self::Blue,
            "dark_blue" => Self::DarkBlue,
            "yellow" => Self::Yellow,
            "dark_yellow" => Self::DarkYellow,
            "magenta" => Self::Magenta,
            "dark_magenta" => Self::DarkMagenta,
            "cyan" => Self::Cyan,
            "dark_cyan" => Self::DarkCyan,
            // Fallback to hex-encoded rgb
            _ => {
                let values = <[u8; 3]>::from_hex(input)?;
                Self::Rgb { r: values[0], g: values[1], b: values[2] }
            }
        };
        Ok(output)
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rgb { r, g, b } => write!(f, "{}", hex::encode([*r, *g, *b])),
            Self::Black => write!(f, "black"),
            Self::White => write!(f, "white"),
            Self::Grey => write!(f, "grey"),
            Self::DarkGrey => write!(f, "dark_grey"),
            Self::Red => write!(f, "red"),
            Self::DarkRed => write!(f, "dark_red"),
            Self::Green => write!(f, "green"),
            Self::DarkGreen => write!(f, "dark_green"),
            Self::Blue => write!(f, "blue"),
            Self::DarkBlue => write!(f, "dark_blue"),
            Self::Yellow => write!(f, "yellow"),
            Self::DarkYellow => write!(f, "dark_yellow"),
            Self::Magenta => write!(f, "magenta"),
            Self::DarkMagenta => write!(f, "dark_magenta"),
            Self::Cyan => write!(f, "cyan"),
            Self::DarkCyan => write!(f, "dark_cyan"),
        }
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

/// Text colors.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct Colors {
    /// The background color.
    pub(crate) background: Option<Color>,

    /// The foreground color.
    pub(crate) foreground: Option<Color>,
}

impl Colors {
    pub(crate) fn merge(&self, other: &Colors) -> Self {
        let background = self.background.or(other.background);
        let foreground = self.foreground.or(other.foreground);
        Self { background, foreground }
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
#[error("invalid color: {0}")]
pub(crate) struct ParseColorError(#[from] FromHexError);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn color_serde() {
        let color: Color = "beef42".parse().unwrap();
        assert_eq!(color.to_string(), "beef42");
    }
}
