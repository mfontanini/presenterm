use crate::theme::ColorPalette;
use crossterm::style::Stylize;
use hex::{FromHex, FromHexError};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Display},
    ops::Deref,
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

    /// Set the background color for this text style.
    pub(crate) fn bg_color(mut self, color: Color) -> Self {
        self.colors.background = Some(color);
        self
    }

    /// Set the foreground color for this text style.
    pub(crate) fn fg_color(mut self, color: Color) -> Self {
        self.colors.foreground = Some(color);
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
    pub(crate) fn apply<T: Into<String>>(&self, text: T) -> Result<<String as Stylize>::Styled, PaletteColorError> {
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
            styled = styled.on(color.try_into()?);
        }
        if let Some(color) = self.colors.foreground {
            styled = styled.with(color.try_into()?);
        }
        Ok(styled)
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
    Palette(FixedStr),
}

impl Color {
    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb { r, g, b }
    }

    pub(crate) fn new_palette(name: &str) -> Result<Self, ParseColorError> {
        let color: FixedStr = name.try_into().map_err(|_| ParseColorError::PaletteColorLength(name.to_string()))?;
        if color.is_empty() { Err(ParseColorError::PaletteColorEmpty) } else { Ok(Self::Palette(color)) }
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

    pub(crate) fn resolve(&self, palette: &ColorPalette) -> Result<Color, UndefinedPaletteColorError> {
        match self {
            Color::Palette(name) => palette.colors.get(name.deref()).cloned().ok_or(UndefinedPaletteColorError(*name)),
            _ => Ok(*self),
        }
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
            other if other.starts_with("palette:") => Self::new_palette(other.trim_start_matches("palette:"))?,
            other if other.starts_with("p:") => Self::new_palette(other.trim_start_matches("p:"))?,
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
            Self::Palette(name) => write!(f, "palette:{name}"),
        }
    }
}

impl TryFrom<Color> for crossterm::style::Color {
    type Error = PaletteColorError;

    fn try_from(value: Color) -> Result<Self, Self::Error> {
        use crossterm::style::Color as C;
        let output = match value {
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
            Color::Palette(color) => return Err(PaletteColorError(color)),
        };
        Ok(output)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FixedStr<const N: usize = 16> {
    data: [u8; N],
    length: u8,
}

impl<const N: usize> TryFrom<&str> for FixedStr<N> {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let data = value.as_bytes();
        if data.len() <= N {
            let mut this = Self { data: [0; N], length: data.len() as u8 };
            this.data[0..data.len()].copy_from_slice(data);
            Ok(this)
        } else {
            Err(())
        }
    }
}

impl<const N: usize> Deref for FixedStr<N> {
    type Target = str;

    fn deref(&self) -> &str {
        let data = &self.data[0..self.length as usize];
        std::str::from_utf8(data).expect("invalid utf8")
    }
}

impl<const N: usize> fmt::Debug for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl<const N: usize> fmt::Display for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unresolved palette color: {0}")]
pub(crate) struct PaletteColorError(FixedStr);

#[derive(Debug, thiserror::Error)]
#[error("undefined palette color: {0}")]
pub(crate) struct UndefinedPaletteColorError(FixedStr);

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

    pub(crate) fn resolve(mut self, palette: &ColorPalette) -> Result<Self, UndefinedPaletteColorError> {
        if let Some(color) = self.foreground.as_mut() {
            *color = color.resolve(palette)?;
        }
        if let Some(color) = self.background.as_mut() {
            *color = color.resolve(palette)?;
        }
        Ok(self)
    }
}

impl TryFrom<Colors> for crossterm::style::Colors {
    type Error = PaletteColorError;

    fn try_from(value: Colors) -> Result<Self, Self::Error> {
        let foreground = value.foreground.map(Color::try_into).transpose()?;
        let background = value.background.map(Color::try_into).transpose()?;
        Ok(Self { foreground, background })
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ParseColorError {
    #[error("invalid hex color: {0}")]
    Hex(#[from] FromHexError),

    #[error("palette color name is too long: {0}")]
    PaletteColorLength(String),

    #[error("palette color name is empty")]
    PaletteColorEmpty,
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[test]
    fn color_serde() {
        let color: Color = "beef42".parse().unwrap();
        assert_eq!(color.to_string(), "beef42");
    }

    #[test]
    fn invalid_fixed_str() {
        FixedStr::<1>::try_from("AB").unwrap_err();
        FixedStr::<1>::try_from("🚀").unwrap_err();
    }

    #[test]
    fn valid_fixed_str() {
        let str = FixedStr::<3>::try_from("ABC").unwrap();
        assert_eq!(str.to_string(), "ABC");
    }

    #[rstest]
    #[case::empty1("p:")]
    #[case::empty2("palette:")]
    #[case::too_long("palette:12345678901234567")]
    fn invalid_palette_color_names(#[case] input: &str) {
        Color::from_str(input).expect_err("not an error");
    }

    #[rstest]
    #[case::short("p:hi", "hi")]
    #[case::long("palette:bye", "bye")]
    fn valid_palette_color_names(#[case] input: &str, #[case] expected: &str) {
        let color = Color::from_str(input).expect("failed to parse");
        let Color::Palette(name) = color else { panic!("not a palette color") };
        assert_eq!(name.deref(), expected);
    }
}
