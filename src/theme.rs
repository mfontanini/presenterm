use crossterm::style::Color;
use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

#[derive(Debug, Deserialize)]
pub struct SlideTheme {
    pub styles: Styles,
}

impl SlideTheme {
    pub fn from_name(name: &str) -> Option<Self> {
        let contents = THEMES.get(name)?;
        // This is going to be caught by the test down here.
        Some(serde_yaml::from_slice(contents).expect("corrupted theme"))
    }

    pub fn alignment(&self, element: &ElementType) -> &Alignment {
        use ElementType::*;

        let alignment = match element {
            SlideTitle => &self.styles.slide_title,
            Heading1 => &self.styles.headings.h1,
            Paragraph => &self.styles.paragraph,
            List => &self.styles.list,
            Code => &self.styles.code.alignment,
            PresentationTitle => &self.styles.presentation.title,
            PresentationSubTitle => &self.styles.presentation.subtitle,
            PresentationAuthor => &self.styles.presentation.author.alignment,
            Table => &self.styles.table,
        };
        alignment.as_ref().unwrap_or(&self.styles.default_style.alignment)
    }
}

#[derive(Debug, Deserialize)]
pub struct Styles {
    #[serde(default, flatten)]
    pub slide_title: Option<Alignment>,

    #[serde(default)]
    pub paragraph: Option<Alignment>,

    #[serde(default)]
    pub code: CodeStyle,

    #[serde(default)]
    pub table: Option<Alignment>,

    #[serde(default)]
    pub list: Option<Alignment>,

    #[serde(rename = "default")]
    pub default_style: PrimaryStyle,

    #[serde(default)]
    pub headings: HeadingStyles,

    #[serde(default)]
    pub presentation: PresentationStyles,

    #[serde(default)]
    pub footer: FooterStyle,
}

#[derive(Debug, Default, Deserialize)]
pub struct HeadingStyles {
    pub h1: Option<Alignment>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PresentationStyles {
    #[serde(default)]
    pub title: Option<Alignment>,

    #[serde(default)]
    pub subtitle: Option<Alignment>,

    pub author: AuthorStyle,
}

#[derive(Debug, Deserialize)]
pub struct PrimaryStyle {
    #[serde(flatten)]
    pub alignment: Alignment,

    pub colors: Colors,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Debug, Default, Deserialize)]
pub struct AuthorStyle {
    #[serde(flatten, default)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub positioning: AuthorPositioning,
}

#[derive(Debug, Default, Deserialize)]
pub struct FooterStyle {
    #[serde(default = "default_footer_template")]
    pub template: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CodeStyle {
    #[serde(flatten)]
    pub alignment: Option<Alignment>,

    #[serde(default)]
    pub colors: Colors,
}

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    SlideTitle,
    Heading1,
    Paragraph,
    List,
    Code,
    PresentationTitle,
    PresentationSubTitle,
    PresentationAuthor,
    Table,
}

#[derive(Debug, Default, Deserialize)]
pub struct Colors {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}

#[derive(Debug, Default, Deserialize)]
pub enum AuthorPositioning {
    BelowTitle,

    #[default]
    PageBottom,
}

fn default_footer_template() -> Option<String> {
    Some("{current_slide} / {total_slides}".to_string())
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
