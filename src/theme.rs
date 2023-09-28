use crossterm::style::Color;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct PresentationTheme {
    #[serde(default)]
    pub slide_title: SlideTitleStyle,

    #[serde(default)]
    pub paragraph: Option<Alignment>,

    #[serde(default)]
    pub code: CodeStyle,

    #[serde(default)]
    pub table: Option<Alignment>,

    #[serde(default)]
    pub list: Option<Alignment>,

    #[serde(default)]
    pub block_quote: BlockQuoteStyle,

    #[serde(rename = "default", default)]
    pub default_style: PrimaryStyle,

    #[serde(default)]
    pub headings: HeadingStyles,

    #[serde(default)]
    pub presentation: PresentationStyles,

    #[serde(default)]
    pub footer: FooterStyle,
}

impl PresentationTheme {
    pub fn from_name(name: &str) -> Option<Self> {
        let contents = THEMES.get(name)?;
        // This is going to be caught by the test down here.
        Some(serde_yaml::from_slice(contents).expect("corrupted theme"))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoadThemeError> {
        let contents = fs::read_to_string(path)?;
        let theme = serde_yaml::from_str(&contents)?;
        Ok(theme)
    }

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
            PresentationTitle => &self.presentation.title,
            PresentationSubTitle => &self.presentation.subtitle,
            PresentationAuthor => &self.presentation.author.alignment,
            Table => &self.table,
            BlockQuote => &self.block_quote.alignment,
        };
        alignment.clone().or_else(|| self.default_style.alignment.clone()).unwrap_or(Alignment::default())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SlideTitleStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub separator: bool,

    #[serde(default)]
    pub padding_top: Option<u8>,

    #[serde(default)]
    pub padding_bottom: Option<u8>,

    #[serde(default)]
    pub colors: Colors,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HeadingStyles {
    #[serde(default)]
    pub h1: HeadingStyle,

    #[serde(default)]
    pub h2: HeadingStyle,

    #[serde(default)]
    pub h3: HeadingStyle,

    #[serde(default)]
    pub h4: HeadingStyle,

    #[serde(default)]
    pub h5: HeadingStyle,

    #[serde(default)]
    pub h6: HeadingStyle,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HeadingStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub prefix: Option<String>,

    #[serde(default)]
    pub colors: Colors,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BlockQuoteStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub colors: Colors,

    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PresentationStyles {
    #[serde(default)]
    pub title: Option<Alignment>,

    #[serde(default)]
    pub subtitle: Option<Alignment>,

    pub author: AuthorStyle,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PrimaryStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    pub colors: Colors,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "alignment", rename_all = "snake_case")]
pub enum Alignment {
    Left {
        #[serde(default)]
        margin: u16,
    },
    Right {
        #[serde(default)]
        margin: u16,
    },
    Center {
        #[serde(default)]
        minimum_margin: u16,

        #[serde(default)]
        minimum_size: u16,
    },
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left { margin: 0 }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AuthorStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub positioning: AuthorPositioning,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum FooterStyle {
    Template { left: Option<String>, right: Option<String> },
    ProgressBar { character: Option<char> },
    Empty,
}

impl Default for FooterStyle {
    fn default() -> Self {
        Self::Template { left: Some("{current_slide} / {total_slides}".to_string()), right: None }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CodeStyle {
    #[serde(flatten)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub colors: Colors,

    #[serde(default)]
    pub padding: Padding,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Padding {
    #[serde(default)]
    pub horizontal: Option<u8>,

    #[serde(default)]
    pub vertical: Option<u8>,
}

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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Colors {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorPositioning {
    BelowTitle,

    #[default]
    PageBottom,
}

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
