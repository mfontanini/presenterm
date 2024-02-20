use crate::style::Colors;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

#[derive(Default)]
pub struct PresentationThemeSet {
    custom_themes: BTreeMap<String, PresentationTheme>,
}

impl PresentationThemeSet {
    /// Loads a theme from its name.
    pub fn load_by_name(&self, name: &str) -> Option<PresentationTheme> {
        if let Some(contents) = THEMES.get(name) {
            // This is going to be caught by the test down here.
            Some(serde_yaml::from_slice(contents).expect("corrupted theme"))
        } else {
            self.custom_themes.get(name).cloned()
        }
    }

    /// Register all the themes in the given directory.
    pub fn register_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), LoadThemeError> {
        let handle = match fs::read_dir(&path) {
            Ok(handle) => handle,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        for entry in handle {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let Some(file_name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };
            if metadata.is_file() && file_name.ends_with(".yaml") {
                let theme_name = file_name.trim_end_matches(".yaml");
                if THEMES.contains_key(theme_name) {
                    return Err(LoadThemeError::Duplicate(theme_name.into()));
                }
                let theme = PresentationTheme::from_path(&entry.path())?;
                self.custom_themes.insert(theme_name.into(), theme);
            }
        }
        Ok(())
    }

    /// Get all the registered theme names.
    pub fn theme_names(&self) -> Vec<String> {
        let builtin_themes = THEMES.keys().map(|name| name.to_string());
        let themes = self.custom_themes.keys().cloned().chain(builtin_themes).collect();
        themes
    }
}

/// A presentation theme.
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationTheme {
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

    /// The style for typst auto-rendered code blocks.
    #[serde(default)]
    pub(crate) modals: ModalStyle,
}

impl PresentationTheme {
    /// Construct a presentation from a path.
    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadThemeError> {
        let contents = fs::read_to_string(&path)?;
        let theme = serde_yaml::from_str(&contents)
            .map_err(|e| LoadThemeError::Corrupted(path.as_ref().display().to_string(), e))?;
        Ok(theme)
    }

    /// Get the alignment for an element.
    ///
    /// This will fall back to the default alignment.
    pub(crate) fn alignment(&self, element: &ElementType) -> Alignment {
        use ElementType::*;

        let alignment = match element {
            SlideTitle => &self.slide_title.alignment,
            Heading1 => &self.headings.h1.alignment,
            Heading2 => &self.headings.h2.alignment,
            Heading3 => &self.headings.h3.alignment,
            Heading4 => &self.headings.h4.alignment,
            Heading5 => &self.headings.h5.alignment,
            Heading6 => &self.headings.h6.alignment,
            Paragraph | List => &None,
            Code => &self.code.alignment,
            PresentationTitle => &self.intro_slide.title.alignment,
            PresentationSubTitle => &self.intro_slide.subtitle.alignment,
            PresentationAuthor => &self.intro_slide.author.alignment,
            Table => &self.table,
            BlockQuote => &self.block_quote.alignment,
        };
        alignment.clone().unwrap_or_default()
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
    pub(crate) bold: bool,

    /// Whether to use italics font for slide titles.
    #[serde(default)]
    pub(crate) italics: bool,

    /// Whether to use underlined font for slide titles.
    #[serde(default)]
    pub(crate) underlined: bool,
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
    pub(crate) colors: Colors,
}

/// The style for the presentation introduction slide.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct IntroSlideStyle {
    /// The style of the title line.
    #[serde(default)]
    pub(crate) title: BasicStyle,

    /// The style of the subtitle line.
    #[serde(default)]
    pub(crate) subtitle: BasicStyle,

    /// The style of the author line.
    #[serde(default)]
    pub(crate) author: AuthorStyle,
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
        /// The template for the text to be put on the left.
        left: Option<String>,

        /// The template for the text to be put on the center.
        center: Option<String>,

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
            center: None,
            right: None,
            colors: Colors::default(),
        }
    }
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
    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: Colors,
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

/// Where to position the author's name in the intro slide.
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

/// An error loading a presentation theme.
#[derive(thiserror::Error, Debug)]
pub enum LoadThemeError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("theme at '{0}' is corrupted: {1}")]
    Corrupted(String, serde_yaml::Error),

    #[error("duplicate custom theme '{0}'")]
    Duplicate(String),
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validate_themes() {
        let themes = PresentationThemeSet::default();
        for theme_name in THEMES.keys() {
            let Some(theme) = themes.load_by_name(theme_name) else {
                panic!("theme '{theme_name}' is corrupted");
            };

            let merged = merge_struct::merge(&PresentationTheme::default(), &theme);
            assert!(merged.is_ok(), "theme '{theme_name}' can't be merged: {}", merged.unwrap_err());
        }
    }

    #[test]
    fn load_custom() {
        let directory = tempdir().expect("creating tempdir");
        let theme = serde_yaml::to_string(&PresentationTheme::default()).unwrap();
        fs::write(directory.path().join("potato.yaml"), &theme).expect("writing theme");

        let mut themes = PresentationThemeSet::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        assert!(themes.load_by_name("potato").is_some());
    }

    #[test]
    fn register_from_missing_directory() {
        let mut themes = PresentationThemeSet::default();
        let result = themes.register_from_directory("/tmp/presenterm/8ee2027983915ec78acc45027d874316");
        result.expect("loading failed");
    }
}
