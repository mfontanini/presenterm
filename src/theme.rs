use crossterm::style::Color;
use serde::Deserialize;
use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

#[derive(Debug, Deserialize)]
pub struct SlideTheme {
    pub default_style: ElementStyle,
    pub element_style: BTreeMap<ElementType, ElementStyle>,
    pub colors: Colors,
    pub author_positioning: AuthorPositioning,
}

impl SlideTheme {
    pub fn from_name(name: &str) -> Option<Self> {
        let contents = THEMES.get(name)?;
        // This is going to be caught by the test down here.
        Some(serde_yaml::from_slice(contents).expect("corrupted theme"))
    }

    pub fn style(&self, element: &ElementType) -> &ElementStyle {
        self.element_style.get(element).unwrap_or(&self.default_style)
    }
}

#[derive(Debug, Deserialize)]
pub struct ElementStyle {
    #[serde(flatten)]
    pub alignment: Alignment,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "alignment")]
pub enum Alignment {
    Left {
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

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
}

#[derive(Debug, Deserialize)]
pub struct Colors {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub code: Option<Color>,
}

#[derive(Debug, Deserialize)]
pub enum AuthorPositioning {
    BelowTitle,
    PageBottom,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn validate_themes() {
        for theme_name in THEMES.keys() {
            assert!(SlideTheme::from_name(theme_name).is_some(), "theme {theme_name} is corrupted");
        }
    }
}
