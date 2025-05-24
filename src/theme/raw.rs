use super::registry::LoadThemeError;
use crate::markdown::text_style::{Color, Colors, UndefinedPaletteColorError};
use hex::{FromHex, FromHexError};
use serde::{Deserialize, Serialize, de::Visitor};
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

pub(crate) type RawColors = Colors<RawColor>;

/// A presentation theme.
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationTheme {
    /// The theme this theme extends from.
    #[serde(default)]
    pub(crate) extends: Option<String>,

    /// The style for a slide's title.
    #[serde(default)]
    pub(super) slide_title: SlideTitleStyle,

    /// The style for a block of code.
    #[serde(default)]
    pub(super) code: CodeBlockStyle,

    /// The style for the execution output of a piece of code.
    #[serde(default)]
    pub(super) execution_output: ExecutionOutputBlockStyle,

    /// The style for inline code.
    #[serde(default)]
    pub(super) inline_code: InlineCodeStyle,

    /// The style for a table.
    #[serde(default)]
    pub(super) table: Option<Alignment>,

    /// The style for a block quote.
    #[serde(default)]
    pub(super) block_quote: BlockQuoteStyle,

    /// The style for an alert.
    #[serde(default)]
    pub(super) alert: AlertStyle,

    /// The default style.
    #[serde(rename = "default", default)]
    pub(super) default_style: DefaultStyle,

    //// The style of all headings.
    #[serde(default)]
    pub(super) headings: HeadingStyles,

    /// The style of the introduction slide.
    #[serde(default)]
    pub(super) intro_slide: IntroSlideStyle,

    /// The style of the presentation footer.
    #[serde(default)]
    pub(super) footer: Option<FooterStyle>,

    /// The style for typst auto-rendered code blocks.
    #[serde(default)]
    pub(super) typst: TypstStyle,

    /// The style for mermaid auto-rendered code blocks.
    #[serde(default)]
    pub(super) mermaid: MermaidStyle,

    /// The style for modals.
    #[serde(default)]
    pub(super) modals: ModalStyle,

    /// The color palette.
    #[serde(default)]
    pub(super) palette: ColorPalette,
}

impl PresentationTheme {
    /// Construct a presentation from a path.
    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadThemeError> {
        let contents = fs::read_to_string(&path)?;
        let theme = serde_yaml::from_str(&contents)
            .map_err(|e| LoadThemeError::Corrupted(path.as_ref().display().to_string(), e.into()))?;
        Ok(theme)
    }
}

/// The style of a slide title.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct SlideTitleStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// Whether to use a separator line.
    #[serde(default)]
    pub(super) separator: bool,

    /// The padding that should be added before the text.
    #[serde(default)]
    pub(super) padding_top: Option<u8>,

    /// The padding that should be added after the text.
    #[serde(default)]
    pub(super) padding_bottom: Option<u8>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// Whether to use bold font for slide titles.
    #[serde(default)]
    pub(super) bold: Option<bool>,

    /// Whether to use italics font for slide titles.
    #[serde(default)]
    pub(super) italics: Option<bool>,

    /// Whether to use underlined font for slide titles.
    #[serde(default)]
    pub(super) underlined: Option<bool>,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(super) font_size: Option<u8>,
}

/// The style for all headings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct HeadingStyles {
    /// H1 style.
    #[serde(default)]
    pub(super) h1: HeadingStyle,

    /// H2 style.
    #[serde(default)]
    pub(super) h2: HeadingStyle,

    /// H3 style.
    #[serde(default)]
    pub(super) h3: HeadingStyle,

    /// H4 style.
    #[serde(default)]
    pub(super) h4: HeadingStyle,

    /// H5 style.
    #[serde(default)]
    pub(super) h5: HeadingStyle,

    /// H6 style.
    #[serde(default)]
    pub(super) h6: HeadingStyle,
}

/// The style for a heading.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct HeadingStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The prefix to be added to this heading.
    ///
    /// This allows adding text like "->" to every heading.
    #[serde(default)]
    pub(super) prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(super) font_size: Option<u8>,
}

/// The style of a block quote.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct BlockQuoteStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub(super) prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: BlockQuoteColors,
}

/// The colors of a block quote.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct BlockQuoteColors {
    /// The foreground/background colors.
    #[serde(flatten)]
    pub(super) base: RawColors,

    /// The color of the vertical bar that prefixes each line in the quote.
    #[serde(default)]
    pub(super) prefix: Option<RawColor>,
}

/// The style of an alert.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct AlertStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The base colors.
    #[serde(default)]
    pub(super) base_colors: RawColors,

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub(super) prefix: Option<String>,

    /// The style for each alert type.
    #[serde(default)]
    pub(super) styles: AlertTypeStyles,
}

/// The style for each alert type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct AlertTypeStyles {
    /// The style for note alert types.
    #[serde(default)]
    pub(super) note: AlertTypeStyle,

    /// The style for tip alert types.
    #[serde(default)]
    pub(super) tip: AlertTypeStyle,

    /// The style for important alert types.
    #[serde(default)]
    pub(super) important: AlertTypeStyle,

    /// The style for warning alert types.
    #[serde(default)]
    pub(super) warning: AlertTypeStyle,

    /// The style for caution alert types.
    #[serde(default)]
    pub(super) caution: AlertTypeStyle,
}

/// The style for an alert type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct AlertTypeStyle {
    /// The color to be used.
    #[serde(default)]
    pub(super) color: Option<RawColor>,

    /// The title to be used.
    #[serde(default)]
    pub(super) title: Option<String>,

    /// The icon to be used.
    #[serde(default)]
    pub(super) icon: Option<String>,
}

/// The style for the presentation introduction slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct IntroSlideStyle {
    /// The style of the title line.
    #[serde(default)]
    pub(super) title: IntroSlideTitleStyle,

    /// The style of the subtitle line.
    #[serde(default)]
    pub(super) subtitle: BasicStyle,

    /// The style of the event line.
    #[serde(default)]
    pub(super) event: BasicStyle,

    /// The style of the location line.
    #[serde(default)]
    pub(super) location: BasicStyle,

    /// The style of the date line.
    #[serde(default)]
    pub(super) date: BasicStyle,

    /// The style of the author line.
    #[serde(default)]
    pub(super) author: AuthorStyle,

    /// Whether we want a footer in the intro slide.
    #[serde(default)]
    pub(super) footer: Option<bool>,
}

/// A simple style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct DefaultStyle {
    /// The margin on the left/right of the screen.
    #[serde(default, with = "serde_yaml::with::singleton_map")]
    pub(super) margin: Option<Margin>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,
}

/// A simple style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct BasicStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,
}

/// The intro slide title's style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct IntroSlideTitleStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(super) font_size: Option<u8>,
}

/// Text alignment.
///
/// This allows anchoring presentation elements to the left, center, or right of the screen.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "alignment", rename_all = "snake_case")]
pub(super) enum Alignment {
    /// Left alignment.
    Left {
        /// The margin before any text.
        #[serde(default)]
        margin: Margin,
    },

    /// Right alignment.
    Right {
        /// The margin after any text.
        #[serde(default)]
        margin: Margin,
    },

    /// Center alignment.
    Center {
        /// The minimum margin expected.
        #[serde(default)]
        minimum_margin: Margin,

        /// The minimum size of this element, in columns.
        #[serde(default)]
        minimum_size: u16,
    },
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left { margin: Margin::Fixed(0) }
    }
}

/// The style for the author line in the presentation intro slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct AuthorStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(super) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// The positioning of the author's name.
    #[serde(default)]
    pub(super) positioning: AuthorPositioning,
}

/// The style of the footer that's shown in every slide.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub(super) enum FooterStyle {
    /// Use a template to generate the footer.
    Template {
        /// The content to be put on the left.
        left: Option<FooterContent>,

        /// The content to be put on the center.
        center: Option<FooterContent>,

        /// The content to be put on the right.
        right: Option<FooterContent>,

        /// The colors to be used.
        #[serde(default)]
        colors: RawColors,

        /// The height of the footer area.
        height: Option<u16>,
    },

    /// Use a progress bar.
    ProgressBar {
        /// The character that will be used for the progress bar.
        character: Option<char>,

        /// The colors to be used.
        #[serde(default)]
        colors: RawColors,
    },

    /// No footer.
    Empty,
}

impl Default for FooterStyle {
    fn default() -> Self {
        Self::Template { left: None, center: None, right: None, colors: RawColors::default(), height: None }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub(crate) enum FooterTemplateChunk {
    Literal(String),
    OpenBrace,
    ClosedBrace,
    CurrentSlide,
    TotalSlides,
    Author,
    Title,
    SubTitle,
    Event,
    Location,
    Date,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(super) enum FooterContent {
    Template(FooterTemplate),
    Image {
        #[serde(rename = "image")]
        path: PathBuf,
    },
}

struct FooterContentVisitor;

impl<'de> Visitor<'de> for FooterContentVisitor {
    type Value = FooterContent;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid footer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let template = FooterTemplate::from_str(v).map_err(|e| E::custom(e.to_string()))?;
        Ok(FooterContent::Template(template))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let Some((key, value)): Option<(String, PathBuf)> = map.next_entry()? else {
            return Err(serde::de::Error::custom("invalid footer"));
        };

        match key.as_str() {
            "image" => Ok(FooterContent::Image { path: value }),
            _ => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(&key), &self)),
        }
    }
}

impl<'de> Deserialize<'de> for FooterContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FooterContentVisitor)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FooterTemplate(pub(crate) Vec<FooterTemplateChunk>);

crate::utils::impl_deserialize_from_str!(FooterTemplate);
crate::utils::impl_serialize_from_display!(FooterTemplate);

impl FromStr for FooterTemplate {
    type Err = ParseFooterTemplateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chunks = Vec::new();
        let mut chunk_start = 0;
        let mut in_variable = false;
        let mut iter = s.char_indices().peekable();
        while let Some((index, c)) = iter.next() {
            if c == '{' {
                if in_variable {
                    return Err(ParseFooterTemplateError::NestedOpenBrace);
                }
                let double_brace = iter.peek() == Some(&(index + 1, '{'));
                if double_brace {
                    iter.next();
                    if chunk_start != index {
                        chunks.push(FooterTemplateChunk::Literal(s[chunk_start..index].to_string()));
                    }
                    chunks.push(FooterTemplateChunk::OpenBrace);
                    chunk_start = index + 2;
                } else {
                    in_variable = true;
                    if chunk_start != index {
                        chunks.push(FooterTemplateChunk::Literal(s[chunk_start..index].to_string()));
                    }
                    chunk_start = index + 1;
                }
            } else if c == '}' {
                if !in_variable {
                    let double_brace = iter.peek() == Some(&(index + 1, '}'));
                    if double_brace {
                        iter.next();
                        chunks.push(FooterTemplateChunk::Literal(s[chunk_start..index].to_string()));
                        chunks.push(FooterTemplateChunk::ClosedBrace);
                        in_variable = false;
                        chunk_start = index + 2;
                        continue;
                    }
                    return Err(ParseFooterTemplateError::ClosedBraceWithoutOpen);
                }
                let variable = &s[chunk_start..index];
                let chunk = match variable {
                    "current_slide" => FooterTemplateChunk::CurrentSlide,
                    "total_slides" => FooterTemplateChunk::TotalSlides,
                    "author" => FooterTemplateChunk::Author,
                    "title" => FooterTemplateChunk::Title,
                    "sub_title" => FooterTemplateChunk::SubTitle,
                    "event" => FooterTemplateChunk::Event,
                    "location" => FooterTemplateChunk::Location,
                    "date" => FooterTemplateChunk::Date,
                    _ => return Err(ParseFooterTemplateError::UnsupportedVariable(variable.to_string())),
                };
                chunks.push(chunk);
                in_variable = false;
                chunk_start = index + 1;
            }
        }
        if in_variable {
            return Err(ParseFooterTemplateError::TrailingBrace);
        } else if chunk_start != s.len() {
            chunks.push(FooterTemplateChunk::Literal(s[chunk_start..].to_string()));
        }
        Ok(Self(chunks))
    }
}

impl fmt::Display for FooterTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FooterTemplateChunk::*;
        for c in &self.0 {
            match c {
                Literal(l) => write!(f, "{l}"),
                OpenBrace => write!(f, "{{{{"),
                ClosedBrace => write!(f, "}}}}"),
                CurrentSlide => write!(f, "{{current_slide}}"),
                TotalSlides => write!(f, "{{total_slides}}"),
                Author => write!(f, "{{author}}"),
                Title => write!(f, "{{title}}"),
                SubTitle => write!(f, "{{sub_title}}"),
                Event => write!(f, "{{event}}"),
                Location => write!(f, "{{location}}"),
                Date => write!(f, "{{date}}"),
            }?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseFooterTemplateError {
    #[error("found '{{' while already inside '{{' scope")]
    NestedOpenBrace,

    #[error("open '{{' was not closed")]
    TrailingBrace,

    #[error("found '}}' but no '{{' was found")]
    ClosedBraceWithoutOpen,

    #[error("unsupported variable: '{0}'")]
    UnsupportedVariable(String),
}

/// The style for a piece of code.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct CodeBlockStyle {
    /// The alignment.
    #[serde(flatten)]
    pub(super) alignment: Option<Alignment>,

    /// The padding.
    #[serde(default)]
    pub(super) padding: PaddingRect,

    /// The syntect theme name to use.
    #[serde(default)]
    pub(super) theme_name: Option<String>,

    /// Whether to use the theme's background color.
    pub(super) background: Option<bool>,
}

/// The style for the output of a code execution block.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ExecutionOutputBlockStyle {
    /// The colors to be used for the output pane.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// The colors to be used for the text that represents the status of the execution block.
    #[serde(default)]
    pub(super) status: ExecutionStatusBlockStyle,

    /// The padding.
    #[serde(default)]
    pub(super) padding: PaddingRect,
}

/// The style for the status of a code execution block.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ExecutionStatusBlockStyle {
    /// The colors for the "running" status.
    #[serde(default)]
    pub(super) running: RawColors,

    /// The colors for the "finished" status.
    #[serde(default)]
    pub(super) success: RawColors,

    /// The colors for the "finished with error" status.
    #[serde(default)]
    pub(super) failure: RawColors,

    /// The colors for the "not started" status.
    #[serde(default)]
    pub(super) not_started: RawColors,
}

/// The style for inline code.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct InlineCodeStyle {
    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,
}

/// Vertical/horizontal padding.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct PaddingRect {
    /// The number of columns to use as horizontal padding.
    #[serde(default)]
    pub(super) horizontal: Option<u8>,

    /// The number of rows to use as vertical padding.
    #[serde(default)]
    pub(super) vertical: Option<u8>,
}

/// A margin.
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Margin {
    /// A fixed number of characters.
    Fixed(u16),

    /// A percent of the screen size.
    Percent(u16),
}

impl Margin {
    pub(crate) fn as_characters(&self, screen_size: u16) -> u16 {
        match *self {
            Self::Fixed(value) => value,
            Self::Percent(percent) => {
                let ratio = percent as f64 / 100.0;
                (screen_size as f64 * ratio).ceil() as u16
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Fixed(0) | Self::Percent(0))
    }
}

impl Default for Margin {
    fn default() -> Self {
        Self::Fixed(0)
    }
}

/// An element type.
#[derive(Clone, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ElementType {
    SlideTitle,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    List,
    Code,
    PresentationTitle,
    PresentationSubTitle,
    PresentationEvent,
    PresentationLocation,
    PresentationDate,
    PresentationAuthor,
    Table,
    BlockQuote,
}

/// Where to position the author's name in the intro slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthorPositioning {
    /// Right below the title.
    BelowTitle,

    /// At the bottom of the page.
    #[default]
    PageBottom,
}

/// Typst styles.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct TypstStyle {
    /// The horizontal margin on the generated images.
    pub(super) horizontal_margin: Option<u16>,

    /// The vertical margin on the generated images.
    pub(super) vertical_margin: Option<u16>,

    /// The colors to be used.
    #[serde(default)]
    pub(super) colors: RawColors,
}

/// Mermaid styles.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct MermaidStyle {
    /// The mermaidjs theme to use.
    pub(super) theme: Option<String>,

    /// The background color to use.
    pub(super) background: Option<String>,
}

/// Modals style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ModalStyle {
    /// The default colors to use for everything in the modal.
    #[serde(default)]
    pub(super) colors: RawColors,

    /// The colors to use for selected lines.
    #[serde(default)]
    pub(super) selection_colors: RawColors,
}

/// The color palette.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ColorPalette {
    #[serde(default)]
    pub(super) colors: BTreeMap<String, RawColor>,

    #[serde(default)]
    pub(super) classes: BTreeMap<String, RawColors>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawColor {
    Color(Color),
    Palette(String),
    ForegroundClass(String),
    BackgroundClass(String),
}

crate::utils::impl_deserialize_from_str!(RawColor);
crate::utils::impl_serialize_from_display!(RawColor);

impl RawColor {
    fn new_palette(name: &str) -> Result<Self, ParseColorError> {
        if name.is_empty() { Err(ParseColorError::PaletteColorEmpty) } else { Ok(Self::Palette(name.into())) }
    }

    pub(crate) fn resolve(
        &self,
        palette: &crate::theme::clean::ColorPalette,
    ) -> Result<Option<Color>, UndefinedPaletteColorError> {
        let color = match self {
            Self::Color(c) => Some(*c),
            Self::Palette(name) => {
                Some(palette.colors.get(name).copied().ok_or(UndefinedPaletteColorError(name.clone()))?)
            }
            Self::ForegroundClass(name) => {
                palette.classes.get(name).ok_or(UndefinedPaletteColorError(name.clone()))?.foreground
            }
            Self::BackgroundClass(name) => {
                palette.classes.get(name).ok_or(UndefinedPaletteColorError(name.clone()))?.background
            }
        };
        Ok(color)
    }
}

impl From<Color> for RawColor {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

impl FromStr for RawColor {
    type Err = ParseColorError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let output = match input {
            "black" => Color::Black.into(),
            "white" => Color::White.into(),
            "grey" => Color::Grey.into(),
            "dark_grey" => Color::DarkGrey.into(),
            "red" => Color::Red.into(),
            "dark_red" => Color::DarkRed.into(),
            "green" => Color::Green.into(),
            "dark_green" => Color::DarkGreen.into(),
            "blue" => Color::Blue.into(),
            "dark_blue" => Color::DarkBlue.into(),
            "yellow" => Color::Yellow.into(),
            "dark_yellow" => Color::DarkYellow.into(),
            "magenta" => Color::Magenta.into(),
            "dark_magenta" => Color::DarkMagenta.into(),
            "cyan" => Color::Cyan.into(),
            "dark_cyan" => Color::DarkCyan.into(),
            other if other.starts_with("palette:") => Self::new_palette(other.trim_start_matches("palette:"))?,
            other if other.starts_with("p:") => Self::new_palette(other.trim_start_matches("p:"))?,
            // Fallback to hex-encoded rgb
            _ => {
                let hex = match input.len() {
                    6 => input.to_string(),
                    3 => input.chars().flat_map(|c| [c, c]).collect::<String>(),
                    len => return Err(ParseColorError::InvalidHexLength(len)),
                };
                let values = <[u8; 3]>::from_hex(hex)?;
                Color::Rgb { r: values[0], g: values[1], b: values[2] }.into()
            }
        };
        Ok(output)
    }
}

impl fmt::Display for RawColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Color::*;
        match self {
            Self::Color(Rgb { r, g, b }) => write!(f, "{}", hex::encode([*r, *g, *b])),
            Self::Color(Black) => write!(f, "black"),
            Self::Color(White) => write!(f, "white"),
            Self::Color(Grey) => write!(f, "grey"),
            Self::Color(DarkGrey) => write!(f, "dark_grey"),
            Self::Color(Red) => write!(f, "red"),
            Self::Color(DarkRed) => write!(f, "dark_red"),
            Self::Color(Green) => write!(f, "green"),
            Self::Color(DarkGreen) => write!(f, "dark_green"),
            Self::Color(Blue) => write!(f, "blue"),
            Self::Color(DarkBlue) => write!(f, "dark_blue"),
            Self::Color(Yellow) => write!(f, "yellow"),
            Self::Color(DarkYellow) => write!(f, "dark_yellow"),
            Self::Color(Magenta) => write!(f, "magenta"),
            Self::Color(DarkMagenta) => write!(f, "dark_magenta"),
            Self::Color(Cyan) => write!(f, "cyan"),
            Self::Color(DarkCyan) => write!(f, "dark_cyan"),
            Self::Palette(name) => write!(f, "palette:{name}"),
            Self::ForegroundClass(_) => Err(fmt::Error),
            Self::BackgroundClass(_) => Err(fmt::Error),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ParseColorError {
    #[error("invalid hex color: {0}")]
    Hex(#[from] FromHexError),

    #[error("hex color should only be 3 or 6 long, got hex string of length {0}")]
    InvalidHexLength(usize),

    #[error("palette color name is empty")]
    PaletteColorEmpty,
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[test]
    fn parse_all_footer_template_variables() {
        use FooterTemplateChunk::*;
        let raw = "hi {current_slide} {total_slides} {author} {title} {sub_title} {event} {location} {event}";
        let t: FooterTemplate = raw.parse().expect("invalid input");
        let expected = vec![
            Literal("hi ".into()),
            CurrentSlide,
            Literal(" ".into()),
            TotalSlides,
            Literal(" ".into()),
            Author,
            Literal(" ".into()),
            Title,
            Literal(" ".into()),
            SubTitle,
            Literal(" ".into()),
            Event,
            Literal(" ".into()),
            Location,
            Literal(" ".into()),
            Event,
        ];
        assert_eq!(t.0, expected);
        assert_eq!(t.to_string(), raw);
    }

    #[test]
    fn parse_double_braces() {
        use FooterTemplateChunk::*;
        let raw = "hi {{beep}} {{author}} {{{{}}}}";
        let t: FooterTemplate = raw.parse().expect("invalid input");
        let merged: String =
            t.0.into_iter()
                .map(|l| match l {
                    Literal(s) => s,
                    OpenBrace => "{".to_string(),
                    ClosedBrace => "}".to_string(),
                    _ => panic!("not a literal"),
                })
                .collect();
        assert_eq!(merged, "hi {beep} {author} {{}}");
    }

    #[rstest]
    #[case::trailing("{author")]
    #[case::close_without_open2("author}")]
    fn invalid_footer_templates(#[case] input: &str) {
        FooterTemplate::from_str(input).expect_err("parse succeeded");
    }

    #[test]
    fn color_serde() {
        let color: RawColor = "beef42".parse().unwrap();
        assert_eq!(color.to_string(), "beef42");

        let short_color: RawColor = "ded".parse().unwrap();
        assert_eq!(short_color.to_string(), "ddeedd");
    }

    #[rstest]
    #[case::empty1("p:")]
    #[case::empty2("palette:")]
    fn invalid_palette_color_names(#[case] input: &str) {
        RawColor::from_str(input).expect_err("not an error");
    }

    #[rstest]
    #[case::short("p:hi", "hi")]
    #[case::long("palette:bye", "bye")]
    fn valid_palette_color_names(#[case] input: &str, #[case] expected: &str) {
        let color = RawColor::from_str(input).expect("failed to parse");
        let RawColor::Palette(name) = color else { panic!("not a palette color") };
        assert_eq!(name, expected);
    }
}
