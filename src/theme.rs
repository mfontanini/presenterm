use std::collections::BTreeMap;

pub struct SlideTheme {
    pub default_style: ElementStyle,
    pub element_style: BTreeMap<ElementType, ElementStyle>,
}

impl SlideTheme {
    pub fn style(&self, element: &ElementType) -> &ElementStyle {
        self.element_style.get(element).unwrap_or(&self.default_style)
    }
}

pub struct ElementStyle {
    pub alignment: Alignment,
}

pub enum Alignment {
    Left { margin: u16 },
    Center { minimum_margin: u16 },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
}
