use super::registry::LoadThemeError;
use crate::markdown::text_style::{Color, Colors, FixedStr};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

/// A presentation theme.
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationTheme {
    /// The theme this theme extends from.
    #[serde(default)]
    pub(crate) extends: Option<String>,

    /// The style for a slide's title.
    #[serde(default)]
    pub(crate) slide_title: SlideTitleStyle,

    /// The style for a block of code.
    #[serde(default)]
    pub(crate) code: CodeBlockStyle,

    /// The style for the execution output of a piece of code.
    #[serde(default)]
    pub(crate) execution_output: ExecutionOutputBlockStyle,

    /// The style for inline code.
    #[serde(default)]
    pub(crate) inline_code: InlineCodeStyle,

    /// The style for a table.
    #[serde(default)]
    pub(crate) table: Option<Alignment>,

    /// The style for a block quote.
    #[serde(default)]
    pub(crate) block_quote: BlockQuoteStyle,

    /// The style for an alert.
    #[serde(default)]
    pub(crate) alert: AlertStyle,

    /// The default style.
    #[serde(rename = "default", default)]
    pub(crate) default_style: DefaultStyle,

    //// The style of all headings.
    #[serde(default)]
    pub(crate) headings: HeadingStyles,

    /// The style of the introduction slide.
    #[serde(default)]
    pub(crate) intro_slide: IntroSlideStyle,

    /// The style of the presentation footer.
    #[serde(default)]
    pub(crate) footer: Option<FooterStyle>,

    /// The style for typst auto-rendered code blocks.
    #[serde(default)]
    pub(crate) typst: TypstStyle,

    /// The style for mermaid auto-rendered code blocks.
    #[serde(default)]
    pub(crate) mermaid: MermaidStyle,

    /// The style for modals.
    #[serde(default)]
    pub(crate) modals: ModalStyle,

    /// The color palette.
    #[serde(default)]
    pub(crate) palette: ColorPalette,
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
pub(crate) struct SlideTitleStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// Whether to use a separator line.
    #[serde(default)]
    pub(crate) separator: bool,

    /// The padding that should be added before the text.
    #[serde(default)]
    pub(crate) padding_top: Option<u8>,

    /// The padding that should be added after the text.
    #[serde(default)]
    pub(crate) padding_bottom: Option<u8>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// Whether to use bold font for slide titles.
    #[serde(default)]
    pub(crate) bold: Option<bool>,

    /// Whether to use italics font for slide titles.
    #[serde(default)]
    pub(crate) italics: Option<bool>,

    /// Whether to use underlined font for slide titles.
    #[serde(default)]
    pub(crate) underlined: Option<bool>,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(crate) font_size: Option<u8>,
}

/// The style for all headings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct HeadingStyles {
    /// H1 style.
    #[serde(default)]
    pub(crate) h1: HeadingStyle,

    /// H2 style.
    #[serde(default)]
    pub(crate) h2: HeadingStyle,

    /// H3 style.
    #[serde(default)]
    pub(crate) h3: HeadingStyle,

    /// H4 style.
    #[serde(default)]
    pub(crate) h4: HeadingStyle,

    /// H5 style.
    #[serde(default)]
    pub(crate) h5: HeadingStyle,

    /// H6 style.
    #[serde(default)]
    pub(crate) h6: HeadingStyle,
}

/// The style for a heading.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct HeadingStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The prefix to be added to this heading.
    ///
    /// This allows adding text like "->" to every heading.
    #[serde(default)]
    pub(crate) prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(crate) font_size: Option<u8>,
}

/// The style of a block quote.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct BlockQuoteStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub(crate) prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: BlockQuoteColors,
}

/// The colors of a block quote.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct BlockQuoteColors {
    /// The foreground/background colors.
    #[serde(flatten)]
    pub(crate) base: Colors,

    /// The color of the vertical bar that prefixes each line in the quote.
    #[serde(default)]
    pub(crate) prefix: Option<Color>,
}

/// The style of an alert.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AlertStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The base colors.
    #[serde(default)]
    pub(crate) base_colors: Colors,

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub(crate) prefix: Option<String>,

    /// The style for each alert type.
    #[serde(default)]
    pub(crate) styles: AlertTypeStyles,
}

/// The style for each alert type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AlertTypeStyles {
    /// The style for note alert types.
    #[serde(default)]
    pub(crate) note: AlertTypeStyle,

    /// The style for tip alert types.
    #[serde(default)]
    pub(crate) tip: AlertTypeStyle,

    /// The style for important alert types.
    #[serde(default)]
    pub(crate) important: AlertTypeStyle,

    /// The style for warning alert types.
    #[serde(default)]
    pub(crate) warning: AlertTypeStyle,

    /// The style for caution alert types.
    #[serde(default)]
    pub(crate) caution: AlertTypeStyle,
}

/// The style for an alert type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AlertTypeStyle {
    /// The color to be used.
    #[serde(default)]
    pub(crate) color: Option<Color>,

    /// The title to be used.
    #[serde(default)]
    pub(crate) title: Option<String>,

    /// The icon to be used.
    #[serde(default)]
    pub(crate) icon: Option<String>,
}

/// The style for the presentation introduction slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct IntroSlideStyle {
    /// The style of the title line.
    #[serde(default)]
    pub(crate) title: IntroSlideTitleStyle,

    /// The style of the subtitle line.
    #[serde(default)]
    pub(crate) subtitle: BasicStyle,

    /// The style of the event line.
    #[serde(default)]
    pub(crate) event: BasicStyle,

    /// The style of the location line.
    #[serde(default)]
    pub(crate) location: BasicStyle,

    /// The style of the date line.
    #[serde(default)]
    pub(crate) date: BasicStyle,

    /// The style of the author line.
    #[serde(default)]
    pub(crate) author: AuthorStyle,

    /// Whether we want a footer in the intro slide.
    #[serde(default)]
    pub(crate) footer: Option<bool>,
}

/// A simple style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct DefaultStyle {
    /// The margin on the left/right of the screen.
    #[serde(default, with = "serde_yaml::with::singleton_map")]
    pub(crate) margin: Option<Margin>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,
}

/// A simple style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct BasicStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,
}

/// The intro slide title's style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct IntroSlideTitleStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// The font size to be used if the terminal supports it.
    #[serde(default)]
    pub(crate) font_size: Option<u8>,
}

/// Text alignment.
///
/// This allows anchoring presentation elements to the left, center, or right of the screen.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "alignment", rename_all = "snake_case")]
pub(crate) enum Alignment {
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
pub(crate) struct AuthorStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub(crate) alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// The positioning of the author's name.
    #[serde(default)]
    pub(crate) positioning: AuthorPositioning,
}

/// The style of the footer that's shown in every slide.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub(crate) enum FooterStyle {
    /// Use a template to generate the footer.
    Template {
        /// The content to be put on the left.
        left: Option<FooterContent>,

        /// The content to be put on the center.
        center: Option<FooterContent>,

        /// The content to be put on the right.
        right: Option<FooterTemplate>,

        /// The colors to be used.
        #[serde(default)]
        colors: Colors,

        /// The height of the footer area.
        height: Option<u16>,
    },

    /// Use a progress bar.
    ProgressBar {
        /// The character that will be used for the progress bar.
        character: Option<char>,

        /// The colors to be used.
        #[serde(default)]
        colors: Colors,
    },

    /// No footer.
    Empty,
}

impl Default for FooterStyle {
    fn default() -> Self {
        Self::Template { left: None, center: None, right: None, colors: Colors::default(), height: None }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub(crate) enum FooterTemplateChunk {
    Literal(String),
    CurrentSlide,
    TotalSlides,
    Author,
    Title,
    SubTitle,
    Event,
    Location,
    Date,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum FooterContent {
    Template(FooterTemplate),
    Image {
        #[serde(rename = "image")]
        path: PathBuf,
    },
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr)]
pub(crate) struct FooterTemplate(pub(crate) Vec<FooterTemplateChunk>);

impl FromStr for FooterTemplate {
    type Err = ParseFooterTemplateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chunks = Vec::new();
        let mut chunk_start = 0;
        let mut in_variable = false;
        for (index, c) in s.char_indices() {
            if c == '{' {
                if in_variable {
                    return Err(ParseFooterTemplateError::NestedOpenBrace);
                }
                if chunk_start != index {
                    chunks.push(FooterTemplateChunk::Literal(s[chunk_start..index].to_string()));
                }
                in_variable = true;
                chunk_start = index + 1;
            } else if c == '}' {
                if !in_variable {
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
pub(crate) struct CodeBlockStyle {
    /// The alignment.
    #[serde(flatten)]
    pub(crate) alignment: Option<Alignment>,

    /// The padding.
    #[serde(default)]
    pub(crate) padding: PaddingRect,

    /// The syntect theme name to use.
    #[serde(default)]
    pub(crate) theme_name: Option<String>,

    /// Whether to use the theme's background color.
    pub(crate) background: Option<bool>,
}

/// The style for the output of a code execution block.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ExecutionOutputBlockStyle {
    /// The colors to be used for the output pane.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// The colors to be used for the text that represents the status of the execution block.
    #[serde(default)]
    pub(crate) status: ExecutionStatusBlockStyle,
}

/// The style for the status of a code execution block.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ExecutionStatusBlockStyle {
    /// The colors for the "running" status.
    #[serde(default)]
    pub(crate) running: Colors,

    /// The colors for the "finished" status.
    #[serde(default)]
    pub(crate) success: Colors,

    /// The colors for the "finished with error" status.
    #[serde(default)]
    pub(crate) failure: Colors,

    /// The colors for the "not started" status.
    #[serde(default)]
    pub(crate) not_started: Colors,
}

/// The style for inline code.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct InlineCodeStyle {
    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,
}

/// Vertical/horizontal padding.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct PaddingRect {
    /// The number of columns to use as horizontal padding.
    #[serde(default)]
    pub(crate) horizontal: Option<u8>,

    /// The number of rows to use as vertical padding.
    #[serde(default)]
    pub(crate) vertical: Option<u8>,
}

/// A margin.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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
pub(crate) enum ElementType {
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
pub(crate) struct TypstStyle {
    /// The horizontal margin on the generated images.
    pub(crate) horizontal_margin: Option<u16>,

    /// The vertical margin on the generated images.
    pub(crate) vertical_margin: Option<u16>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,
}

/// Mermaid styles.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct MermaidStyle {
    /// The mermaidjs theme to use.
    pub(crate) theme: Option<String>,

    /// The background color to use.
    pub(crate) background: Option<String>,
}

/// Modals style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ModalStyle {
    /// The default colors to use for everything in the modal.
    #[serde(default)]
    pub(crate) colors: Colors,

    /// The colors to use for selected lines.
    #[serde(default)]
    pub(crate) selection_colors: Colors,
}

/// The color palette.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ColorPalette {
    #[serde(default)]
    pub(crate) colors: BTreeMap<FixedStr, Color>,
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

    #[rstest]
    #[case::nested_open("{{author}")]
    #[case::trailing("{author")]
    #[case::close_without_open1("{author}}")]
    #[case::close_without_open2("author}")]
    fn invalid_footer_templates(#[case] input: &str) {
        FooterTemplate::from_str(input).expect_err("parse succeeded");
    }
}
