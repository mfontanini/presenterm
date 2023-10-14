use crossterm::style::Color;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

/// A presentation theme.
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct PresentationTheme {
    /// The style for a slide's title.
    #[serde(default)]
    pub slide_title: SlideTitleStyle,

    /// The style for a paragraph.
    #[serde(default)]
    pub paragraph: Option<Alignment>,

    /// The style for a block of code.
    #[serde(default)]
    pub code: CodeBlockStyle,

    /// The style for inline code.
    #[serde(default)]
    pub inline_code: InlineCodeStyle,

    /// The style for a table.
    #[serde(default)]
    pub table: Option<Alignment>,

    /// The style for a list.
    #[serde(default)]
    pub list: Option<Alignment>,

    /// The style for a block quote.
    #[serde(default)]
    pub block_quote: BlockQuoteStyle,

    /// The default style.
    ///
    /// This is used as a fallback for any elements that don't have an explicit style.
    #[serde(rename = "default", default)]
    pub default_style: BasicStyle,

    //// The style of all headings.
    #[serde(default)]
    pub headings: HeadingStyles,

    /// The style of the introduction slide.
    #[serde(default)]
    pub intro_slide: IntroSlideStyle,

    /// The style of the presentation footer.
    #[serde(default)]
    pub footer: FooterStyle,
}

impl PresentationTheme {
    /// Get a presentation theme by name.
    ///
    /// Default themes are bundled into the final binary during build time so this is an in-memory
    /// lookup.
    pub fn from_name(name: &str) -> Option<Self> {
        let contents = THEMES.get(name)?;
        // This is going to be caught by the test down here.
        Some(serde_yaml::from_slice(contents).expect("corrupted theme"))
    }

    pub fn theme_names() -> impl Iterator<Item = &'static str> {
        THEMES.keys().copied()
    }

    /// Construct a presentation from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadThemeError> {
        let contents = fs::read_to_string(path)?;
        let theme = serde_yaml::from_str(&contents)?;
        Ok(theme)
    }

    /// Get the alignment for an element.
    ///
    /// This will fall back to the default alignment.
    pub fn alignment(&self, element: &ElementType) -> Alignment {
        use ElementType::*;

        let alignment = match element {
            SlideTitle => &self.slide_title.alignment,
            Heading1 => &self.headings.h1.alignment,
            Heading2 => &self.headings.h2.alignment,
            Heading3 => &self.headings.h3.alignment,
            Heading4 => &self.headings.h4.alignment,
            Heading5 => &self.headings.h5.alignment,
            Heading6 => &self.headings.h6.alignment,
            Paragraph => &self.paragraph,
            List => &self.list,
            Code => &self.code.alignment,
            PresentationTitle => &self.intro_slide.title.alignment,
            PresentationSubTitle => &self.intro_slide.subtitle.alignment,
            PresentationAuthor => &self.intro_slide.author.alignment,
            Table => &self.table,
            BlockQuote => &self.block_quote.alignment,
        };
        alignment.clone().or_else(|| self.default_style.alignment.clone()).unwrap_or(Alignment::default())
    }
}

/// The style of a slide title.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SlideTitleStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    /// Whether to use a separator line.
    #[serde(default)]
    pub separator: bool,

    /// The padding that should be added before the text.
    #[serde(default)]
    pub padding_top: Option<u8>,

    /// The padding that should be added after the text.
    #[serde(default)]
    pub padding_bottom: Option<u8>,

    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,
}

/// The style for all headings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HeadingStyles {
    /// H1 style.
    #[serde(default)]
    pub h1: HeadingStyle,

    /// H2 style.
    #[serde(default)]
    pub h2: HeadingStyle,

    /// H3 style.
    #[serde(default)]
    pub h3: HeadingStyle,

    /// H4 style.
    #[serde(default)]
    pub h4: HeadingStyle,

    /// H5 style.
    #[serde(default)]
    pub h5: HeadingStyle,

    /// H6 style.
    #[serde(default)]
    pub h6: HeadingStyle,
}

/// The style for a heading.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HeadingStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    /// The prefix to be added to this heading.
    ///
    /// This allows adding text like "->" to every heading.
    #[serde(default)]
    pub prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,
}

/// The style of a block quote.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BlockQuoteStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,
}

/// The style for the presentation introduction slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct IntroSlideStyle {
    /// The style of the title line.
    #[serde(default)]
    pub title: BasicStyle,

    /// The style of the subtitle line.
    #[serde(default)]
    pub subtitle: BasicStyle,

    /// The style of the author line.
    #[serde(default)]
    pub author: AuthorStyle,
}

/// A simple style.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BasicStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,
}

/// Text alignment.
///
/// This allows anchoring presentation elements to the left, center, or right of the screen.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "alignment", rename_all = "snake_case")]
pub enum Alignment {
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
pub struct AuthorStyle {
    /// The alignment.
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,

    /// The positioning of the author's name.
    #[serde(default)]
    pub positioning: AuthorPositioning,
}

/// The style of the footer that's shown in every slide.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum FooterStyle {
    /// Use a template to generate the footer.
    Template {
        /// The template for the text to be put on the left.
        left: Option<String>,

        /// The template for the text to be put on the right.
        right: Option<String>,

        /// The colors to be used.
        #[serde(default)]
        colors: Colors,
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
        Self::Template {
            left: Some("{current_slide} / {total_slides}".to_string()),
            right: None,
            colors: Colors::default(),
        }
    }
}

/// The style for a piece of code.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CodeBlockStyle {
    /// The alignment.
    #[serde(flatten)]
    pub alignment: Option<Alignment>,

    /// The padding.
    #[serde(default)]
    pub padding: PaddingRect,

    /// The syntect theme name to use.
    #[serde(default)]
    pub theme_name: Option<String>,
}

/// The style for inline code.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InlineCodeStyle {
    /// The colors to be used.
    #[serde(default)]
    pub colors: Colors,
}

/// Vertical/horizontal padding.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PaddingRect {
    /// The number of columns to use as horizontal padding.
    #[serde(default)]
    pub horizontal: Option<u8>,

    /// The number of rows to use as vertical padding.
    #[serde(default)]
    pub vertical: Option<u8>,
}

/// A margin.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Margin {
    /// A fixed number of characters.
    Fixed(u16),

    /// A percent of the screen size.
    Percent(u16),
}

impl Margin {
    pub fn as_characters(&self, screen_size: u16) -> u16 {
        match *self {
            Self::Fixed(value) => value,
            Self::Percent(percent) => {
                let ratio = percent as f64 / 100.0;
                (screen_size as f64 * ratio).ceil() as u16
            }
        }
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
pub enum ElementType {
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
    PresentationAuthor,
    Table,
    BlockQuote,
}

/// Text colors.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Colors {
    /// The background color.
    pub background: Option<Color>,

    /// The foreground color.
    pub foreground: Option<Color>,
}

/// Where to position the author's name in the intro slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorPositioning {
    /// Right below the title.
    BelowTitle,

    /// At the bottom of the page.
    #[default]
    PageBottom,
}

/// An error loading a presentation theme.
#[derive(thiserror::Error, Debug)]
pub enum LoadThemeError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Corrupted(#[from] serde_yaml::Error),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn validate_themes() {
        for theme_name in THEMES.keys() {
            assert!(PresentationTheme::from_name(theme_name).is_some(), "theme {theme_name} is corrupted");
        }
    }
}
