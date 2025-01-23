use crate::markdown::text_style::{Color, Colors};
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
        let mut dependencies = BTreeMap::new();
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
                let theme = PresentationTheme::from_path(entry.path())?;
                let base = theme.extends.clone();
                self.custom_themes.insert(theme_name.into(), theme);
                dependencies.insert(theme_name.to_string(), base);
            }
        }
        let mut graph = ThemeGraph::new(dependencies);
        for theme_name in graph.dependents.keys() {
            let theme_name = theme_name.as_str();
            if !THEMES.contains_key(theme_name) && !self.custom_themes.contains_key(theme_name) {
                return Err(LoadThemeError::ExtendedThemeNotFound(theme_name.into()));
            }
        }

        while let Some(theme_name) = graph.pop() {
            self.extend_theme(&theme_name)?;
        }
        if !graph.dependents.is_empty() {
            return Err(LoadThemeError::ExtensionLoop(graph.dependents.into_keys().collect()));
        }
        Ok(())
    }

    fn extend_theme(&mut self, theme_name: &str) -> Result<(), LoadThemeError> {
        let Some(base_name) = self.custom_themes.get(theme_name).expect("theme not found").extends.clone() else {
            return Ok(());
        };
        let Some(base_theme) = self.load_by_name(&base_name) else {
            return Err(LoadThemeError::ExtendedThemeNotFound(base_name.clone()));
        };
        let theme = self.custom_themes.get_mut(theme_name).expect("theme not found");
        *theme = merge_struct::merge(&base_theme, theme)
            .map_err(|e| LoadThemeError::Corrupted(base_name.to_string(), e.into()))?;
        Ok(())
    }

    /// Get all the registered theme names.
    pub fn theme_names(&self) -> Vec<String> {
        let builtin_themes = THEMES.keys().map(|name| name.to_string());
        let themes = self.custom_themes.keys().cloned().chain(builtin_themes).collect();
        themes
    }
}

struct ThemeGraph {
    dependents: BTreeMap<String, Vec<String>>,
    ready: Vec<String>,
}

impl ThemeGraph {
    fn new<I>(dependencies: I) -> Self
    where
        I: IntoIterator<Item = (String, Option<String>)>,
    {
        let mut dependents: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut ready = Vec::new();
        for (name, extends) in dependencies {
            dependents.entry(name.clone()).or_default();
            match extends {
                // If we extend from a non built in theme, make ourselves their dependent
                Some(base) if !THEMES.contains_key(base.as_str()) => {
                    dependents.entry(base).or_default().push(name);
                }
                // Otherwise this theme is ready to be processed
                _ => ready.push(name),
            }
        }
        Self { dependents, ready }
    }

    fn pop(&mut self) -> Option<String> {
        let theme = self.ready.pop()?;
        if let Some(dependents) = self.dependents.remove(&theme) {
            self.ready.extend(dependents);
        }
        Some(theme)
    }
}

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
}

impl PresentationTheme {
    /// Construct a presentation from a path.
    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadThemeError> {
        let contents = fs::read_to_string(&path)?;
        let theme = serde_yaml::from_str(&contents)
            .map_err(|e| LoadThemeError::Corrupted(path.as_ref().display().to_string(), e.into()))?;
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
            PresentationEvent => &self.intro_slide.event.alignment,
            PresentationLocation => &self.intro_slide.location.alignment,
            PresentationDate => &self.intro_slide.date.alignment,
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
    pub(crate) bold: Option<bool>,

    /// Whether to use italics font for slide titles.
    #[serde(default)]
    pub(crate) italics: Option<bool>,

    /// Whether to use underlined font for slide titles.
    #[serde(default)]
    pub(crate) underlined: Option<bool>,
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

    /// The prefix to be added to this block quote.
    ///
    /// This allows adding something like a vertical bar before the text.
    #[serde(default)]
    pub(crate) prefix: Option<String>,

    /// The colors to be used.
    #[serde(default)]
    pub(crate) colors: AlertColors,
}

/// The colors of an alert.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AlertColors {
    /// The foreground/background colors.
    #[serde(flatten)]
    pub(crate) base: Colors,

    /// The color of the vertical bar that prefixes each line in the quote.
    #[serde(default)]
    pub(crate) types: AlertTypeColors,
}

/// The colors of each alert type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AlertTypeColors {
    /// The color for note type alerts.
    #[serde(default)]
    pub(crate) note: Option<Color>,

    /// The color for tip type alerts.
    #[serde(default)]
    pub(crate) tip: Option<Color>,

    /// The color for important type alerts.
    #[serde(default)]
    pub(crate) important: Option<Color>,

    /// The color for warning type alerts.
    #[serde(default)]
    pub(crate) warning: Option<Color>,

    /// The color for caution type alerts.
    #[serde(default)]
    pub(crate) caution: Option<Color>,
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
        Self::Template { left: None, center: None, right: None, colors: Colors::default() }
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

/// An error loading a presentation theme.
#[derive(thiserror::Error, Debug)]
pub enum LoadThemeError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("theme at '{0}' is corrupted: {1}")]
    Corrupted(String, Box<dyn std::error::Error>),

    #[error("duplicate custom theme '{0}'")]
    Duplicate(String),

    #[error("extended theme does not exist: {0}")]
    ExtendedThemeNotFound(String),

    #[error("theme has an extension loop involving: {0:?}")]
    ExtensionLoop(Vec<String>),
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::{TempDir, tempdir};

    fn write_theme(name: &str, theme: PresentationTheme, directory: &TempDir) {
        let theme = serde_yaml::to_string(&theme).unwrap();
        let file_name = format!("{name}.yaml");
        fs::write(directory.path().join(file_name), theme).expect("writing theme");
    }

    #[test]
    fn validate_themes() {
        let themes = PresentationThemeSet::default();
        for theme_name in THEMES.keys() {
            let Some(theme) = themes.load_by_name(theme_name) else {
                panic!("theme '{theme_name}' is corrupted");
            };

            // Built-in themes can't use this because... I don't feel like supporting this now.
            assert!(theme.extends.is_none(), "theme '{theme_name}' uses extends");

            let merged = merge_struct::merge(&PresentationTheme::default(), &theme);
            assert!(merged.is_ok(), "theme '{theme_name}' can't be merged: {}", merged.unwrap_err());
        }
    }

    #[test]
    fn load_custom() {
        let directory = tempdir().expect("creating tempdir");
        write_theme(
            "potato",
            PresentationTheme { extends: Some("dark".to_string()), ..Default::default() },
            &directory,
        );

        let mut themes = PresentationThemeSet::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        let mut theme = themes.load_by_name("potato").expect("theme not found");

        // Since we extend the dark theme they must match after we remove the "extends" field.
        let dark = themes.load_by_name("dark");
        theme.extends.take().expect("no extends");
        assert_eq!(serde_yaml::to_string(&theme).unwrap(), serde_yaml::to_string(&dark).unwrap());
    }

    #[test]
    fn load_derive_chain() {
        let directory = tempdir().expect("creating tempdir");
        write_theme("A", PresentationTheme { extends: Some("dark".to_string()), ..Default::default() }, &directory);
        write_theme("B", PresentationTheme { extends: Some("C".to_string()), ..Default::default() }, &directory);
        write_theme("C", PresentationTheme { extends: Some("A".to_string()), ..Default::default() }, &directory);
        write_theme("D", PresentationTheme::default(), &directory);

        let mut themes = PresentationThemeSet::default();
        themes.register_from_directory(directory.path()).expect("loading themes");
        themes.load_by_name("A").expect("A not found");
        themes.load_by_name("B").expect("B not found");
        themes.load_by_name("C").expect("C not found");
        themes.load_by_name("D").expect("D not found");
    }

    #[test]
    fn invalid_derives() {
        let directory = tempdir().expect("creating tempdir");
        write_theme(
            "A",
            PresentationTheme { extends: Some("non-existent-theme".to_string()), ..Default::default() },
            &directory,
        );

        let mut themes = PresentationThemeSet::default();
        themes.register_from_directory(directory.path()).expect_err("loading themes succeeded");
    }

    #[test]
    fn load_derive_chain_loop() {
        let directory = tempdir().expect("creating tempdir");
        write_theme("A", PresentationTheme { extends: Some("B".to_string()), ..Default::default() }, &directory);
        write_theme("B", PresentationTheme { extends: Some("A".to_string()), ..Default::default() }, &directory);

        let mut themes = PresentationThemeSet::default();
        let err = themes.register_from_directory(directory.path()).expect_err("loading themes succeeded");
        let LoadThemeError::ExtensionLoop(names) = err else { panic!("not an extension loop error") };
        assert_eq!(names, &["A", "B"]);
    }

    #[test]
    fn register_from_missing_directory() {
        let mut themes = PresentationThemeSet::default();
        let result = themes.register_from_directory("/tmp/presenterm/8ee2027983915ec78acc45027d874316");
        result.expect("loading failed");
    }
}
