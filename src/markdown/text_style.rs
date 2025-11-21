use crate::{
    terminal::capabilities::TerminalCapabilities,
    theme::{ColorPalette, raw::RawColor},
};
use crossterm::style::{ContentStyle, StyledContent, Stylize};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

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

    /// Indicate this is a superscript.
    pub(crate) fn superscript(self) -> Self {
        self.add_flag(TextFormatFlags::Superscript)
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

    /// Check whether this text is code.
    pub(crate) fn is_code(&self) -> bool {
        self.has_flag(TextFormatFlags::Code)
    }

    /// Check whether this text is bold.
    pub(crate) fn is_bold(&self) -> bool {
        self.has_flag(TextFormatFlags::Bold)
    }

    /// Check whether this text is italics.
    pub(crate) fn is_italics(&self) -> bool {
        self.has_flag(TextFormatFlags::Italics)
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
    pub(crate) fn apply<'a>(
        &self,
        text: &'a str,
        capabilities: &TerminalCapabilities,
    ) -> StyledContent<impl Display + Clone + 'a> {
        let mut contents = Cow::Borrowed(text);
        let mut font_size = FontSize::Scaled(self.size);
        let mut style = ContentStyle::default();
        for attr in self.iter_attributes() {
            style = match attr {
                TextAttribute::Bold => style.bold(),
                TextAttribute::Italics => style.italic(),
                TextAttribute::Strikethrough => style.crossed_out(),
                TextAttribute::Underlined => style.underlined(),
                TextAttribute::Superscript => {
                    if capabilities.fractional_font_size {
                        font_size = FontSize::Fractional { numerator: self.size, denominator: 2 }
                    } else if let Some(t) = text.try_into_superscript() {
                        contents = Cow::Owned(t);
                    }
                    style
                }
                TextAttribute::ForegroundColor(color) => style.with(color.into()),
                TextAttribute::BackgroundColor(color) => style.on(color.into()),
            }
        }
        let text = FontSizedStr { contents, font_size };
        StyledContent::new(style, text)
    }

    pub(crate) fn into_raw(self) -> TextStyle<RawColor> {
        let colors = Colors {
            background: self.colors.background.map(Into::into),
            foreground: self.colors.foreground.map(Into::into),
        };
        TextStyle { flags: self.flags, colors, size: self.size }
    }

    /// Iterate all attributes in this style.
    pub(crate) fn iter_attributes(&self) -> AttributeIterator {
        AttributeIterator {
            flags: self.flags,
            next_mask: Some(TextFormatFlags::Bold),
            background_color: self.colors.background,
            foreground_color: self.colors.foreground,
        }
    }
}

impl TextStyle<RawColor> {
    pub(crate) fn resolve(&self, palette: &ColorPalette) -> Result<TextStyle, UndefinedPaletteColorError> {
        let colors = self.colors.resolve(palette)?;
        Ok(TextStyle { flags: self.flags, colors, size: self.size })
    }
}

pub(crate) struct AttributeIterator {
    flags: u8,
    next_mask: Option<TextFormatFlags>,
    background_color: Option<Color>,
    foreground_color: Option<Color>,
}

impl Iterator for AttributeIterator {
    type Item = TextAttribute;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(c) = self.background_color.take() {
            return Some(TextAttribute::BackgroundColor(c));
        }
        if let Some(c) = self.foreground_color.take() {
            return Some(TextAttribute::ForegroundColor(c));
        }
        use TextFormatFlags::*;
        loop {
            let next_mask = self.next_mask?;
            self.next_mask = match next_mask {
                Bold => Some(Italics),
                Italics => Some(Strikethrough),
                Code => Some(Strikethrough),
                Strikethrough => Some(Superscript),
                Superscript => Some(Underlined),
                Underlined => None,
            };
            if self.flags & next_mask as u8 != 0 {
                let attr = match next_mask {
                    Bold => TextAttribute::Bold,
                    Italics => TextAttribute::Italics,
                    Code => panic!("code shouldn't reach here"),
                    Strikethrough => TextAttribute::Strikethrough,
                    Superscript => TextAttribute::Superscript,
                    Underlined => TextAttribute::Underlined,
                };
                return Some(attr);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextAttribute {
    Bold,
    Italics,
    Strikethrough,
    Underlined,
    Superscript,
    ForegroundColor(Color),
    BackgroundColor(Color),
}

#[derive(Clone, Debug)]
struct FontSizedStr<'a> {
    contents: Cow<'a, str>,
    font_size: FontSize,
}

impl fmt::Display for FontSizedStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = &self.contents;
        match self.font_size {
            FontSize::Scaled(0 | 1) => write!(f, "{contents}"),
            FontSize::Scaled(size) => write!(f, "\x1b]66;s={size};{contents}\x1b\\"),
            FontSize::Fractional { numerator, denominator } => {
                write!(f, "\x1b]66;n={numerator}:d={denominator};{contents}\x1b\\")
            }
        }
    }
}

#[derive(Clone, Debug)]
enum FontSize {
    Scaled(u8),
    Fractional { numerator: u8, denominator: u8 },
}

#[derive(Clone, Copy, Debug)]
enum TextFormatFlags {
    Bold = 1,
    Italics = 2,
    Code = 4,
    Strikethrough = 8,
    Underlined = 16,
    Superscript = 32,
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

    pub(crate) fn from_8bit(color: u8) -> Option<Self> {
        match color {
            0 => Self::Black.into(),
            1 => Self::DarkRed.into(),
            2 => Self::DarkGreen.into(),
            3 => Self::DarkYellow.into(),
            4 => Self::DarkBlue.into(),
            5 => Self::DarkMagenta.into(),
            6 => Self::DarkCyan.into(),
            7 => Self::Grey.into(),
            8 => Self::DarkGrey.into(),
            9 => Self::Red.into(),
            10 => Self::Green.into(),
            11 => Self::Yellow.into(),
            12 => Self::Blue.into(),
            13 => Self::Magenta.into(),
            14 => Self::Cyan.into(),
            15 => Self::White.into(),
            16..=231 => {
                let mapping = [0, 95, 95 + 40, 95 + 80, 95 + 120, 95 + 160];
                let mut value = color - 16;
                let b = (value % 6) as usize;
                value /= 6;
                let g = (value % 6) as usize;
                value /= 6;
                let r = (value % 6) as usize;
                Some(Self::new(mapping[r], mapping[g], mapping[b]))
            }
            _ => None,
        }
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

trait TryIntoSuperscript {
    fn try_into_superscript(&self) -> Option<String>;
}

impl TryIntoSuperscript for &'_ str {
    fn try_into_superscript(&self) -> Option<String> {
        let mut output = String::new();
        for from in self.chars() {
            let to = match from {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                '+' => '⁺',
                '-' => '⁻',
                '=' => '⁼',
                '(' => '⁽',
                ')' => '⁾',
                'a' => 'ᵃ',
                'b' => 'ᵇ',
                'c' => 'ᶜ',
                'd' => 'ᵈ',
                'e' => 'ᵉ',
                'f' => 'ᶠ',
                'g' => 'ᵍ',
                'h' => 'ʰ',
                'i' => 'ⁱ',
                'j' => 'ʲ',
                'k' => 'ᵏ',
                'l' => 'ˡ',
                'm' => 'ᵐ',
                'n' => 'ⁿ',
                'o' => 'ᵒ',
                'p' => 'ᵖ',
                'q' => '𐞥',
                'r' => 'ʳ',
                's' => 'ˢ',
                't' => 'ᵗ',
                'u' => 'ᵘ',
                'v' => 'ᵛ',
                'w' => 'ʷ',
                'x' => 'ˣ',
                'y' => 'ʸ',
                'z' => 'ᶻ',
                _ => return None,
            };
            output.push(to);
        }
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::default(TextStyle::default(), &[])]
    #[case::code(TextStyle::default().code(), &[])]
    #[case::bold(TextStyle::default().bold(), &[TextAttribute::Bold])]
    #[case::italics(TextStyle::default().italics(), &[TextAttribute::Italics])]
    #[case::strikethrough(TextStyle::default().strikethrough(), &[TextAttribute::Strikethrough])]
    #[case::underlined(TextStyle::default().underlined(), &[TextAttribute::Underlined])]
    #[case::bg_color(TextStyle::default().bg_color(Color::Red), &[TextAttribute::BackgroundColor(Color::Red)])]
    #[case::bg_color(TextStyle::default().fg_color(Color::Red), &[TextAttribute::ForegroundColor(Color::Red)])]
    #[case::all(
        TextStyle::default().bold().code().italics().strikethrough().underlined().bg_color(Color::Black).fg_color(Color::Red),
        &[
            TextAttribute::BackgroundColor(Color::Black),
            TextAttribute::ForegroundColor(Color::Red),
            TextAttribute::Bold,
            TextAttribute::Italics,
            TextAttribute::Strikethrough,
            TextAttribute::Underlined,
        ]
    )]
    fn iterate_attributes(#[case] style: TextStyle, #[case] expected: &[TextAttribute]) {
        let attrs: Vec<_> = style.iter_attributes().collect();
        assert_eq!(attrs, expected);
    }
}
