use crossterm::style::Stylize;
use hex::{FromHex, FromHexError};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

/// The style of a piece of text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextStyle {
    flags: u8,
    pub(crate) colors: Colors,
}

impl TextStyle {
    /// Add bold to this style.
    pub(crate) fn bold(mut self) -> Self {
        self.flags |= TextFormatFlags::Bold as u8;
        self
    }

    /// Add italics to this style.
    pub(crate) fn italics(mut self) -> Self {
        self.flags |= TextFormatFlags::Italics as u8;
        self
    }

    /// Indicate this text is a piece of inline code.
    pub(crate) fn code(mut self) -> Self {
        self.flags |= TextFormatFlags::Code as u8;
        self
    }

    /// Add strikethrough to this style.
    pub(crate) fn strikethrough(mut self) -> Self {
        self.flags |= TextFormatFlags::Strikethrough as u8;
        self
    }

    /// Indicate this is a link.
    pub(crate) fn link(mut self) -> Self {
        self.flags |= TextFormatFlags::Link as u8;
        self
    }

    /// Set the colors for this text style.
    pub(crate) fn colors(mut self, colors: Colors) -> Self {
        self.colors = colors;
        self
    }

    /// Check whether this text style is bold.
    pub(crate) fn is_bold(&self) -> bool {
        self.flags & TextFormatFlags::Bold as u8 != 0
    }

    /// Check whether this text style has italics.
    pub(crate) fn is_italics(&self) -> bool {
        self.flags & TextFormatFlags::Italics as u8 != 0
    }

    /// Check whether this text is code.
    pub(crate) fn is_code(&self) -> bool {
        self.flags & TextFormatFlags::Code as u8 != 0
    }

    /// Check whether this text style is strikethrough.
    pub(crate) fn is_strikethrough(&self) -> bool {
        self.flags & TextFormatFlags::Strikethrough as u8 != 0
    }

    /// Check whether this text is a link.
    pub(crate) fn is_link(&self) -> bool {
        self.flags & TextFormatFlags::Link as u8 != 0
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
        if self.is_link() {
            styled = styled.italic().underlined();
        }
        if let Some(color) = self.colors.background {
            styled = styled.on(color.into());
        }
        if let Some(color) = self.colors.foreground {
            styled = styled.with(color.into());
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub(crate) struct Color(crossterm::style::Color);

impl Color {
    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self(crossterm::style::Color::Rgb { r, g, b })
    }

    pub(crate) fn as_rgb(&self) -> Option<(u8, u8, u8)> {
        match self.0 {
            crossterm::style::Color::Rgb { r, g, b } => Some((r, g, b)),
            _ => None,
        }
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        use crossterm::style::Color as C;
        let output = match input {
            "black" => Self(C::Black),
            "white" => Self(C::White),
            "grey" => Self(C::Grey),
            "red" => Self(C::Red),
            "dark_red" => Self(C::DarkRed),
            "green" => Self(C::Green),
            "dark_green" => Self(C::DarkGreen),
            "blue" => Self(C::Blue),
            "dark_blue" => Self(C::DarkBlue),
            "yellow" => Self(C::Yellow),
            "dark_yellow" => Self(C::DarkYellow),
            "magenta" => Self(C::Magenta),
            "dark_magenta" => Self(C::DarkMagenta),
            "cyan" => Self(C::Cyan),
            "dark_cyan" => Self(C::DarkCyan),
            // Fallback to hex-encoded rgb
            _ => {
                let values = <[u8; 3]>::from_hex(input)?;
                Self(crossterm::style::Color::Rgb { r: values[0], g: values[1], b: values[2] })
            }
        };
        Ok(output)
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crossterm::style::Color as C;
        match self.0 {
            C::Rgb { r, g, b } => write!(f, "{}", hex::encode([r, g, b])),
            C::Black => write!(f, "black"),
            C::White => write!(f, "white"),
            C::Grey => write!(f, "grey"),
            C::Red => write!(f, "red"),
            C::DarkRed => write!(f, "dark_red"),
            C::Green => write!(f, "green"),
            C::DarkGreen => write!(f, "dark_green"),
            C::Blue => write!(f, "blue"),
            C::DarkBlue => write!(f, "dark_blue"),
            C::Yellow => write!(f, "yellow"),
            C::DarkYellow => write!(f, "dark_yellow"),
            C::Magenta => write!(f, "magenta"),
            C::DarkMagenta => write!(f, "dark_magenta"),
            C::Cyan => write!(f, "cyan"),
            C::DarkCyan => write!(f, "dark_cyan"),
            _ => panic!("unsupported color"),
        }
    }
}

impl From<Color> for crossterm::style::Color {
    fn from(value: Color) -> Self {
        value.0
    }
}

/// Text colors.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct Colors {
    /// The background color.
    pub(crate) background: Option<Color>,

    /// The foreground color.
    pub(crate) foreground: Option<Color>,
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
