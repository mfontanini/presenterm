use crate::elements::Element;

#[derive(Debug)]
pub struct Slide {
    pub elements: Vec<Element>,
}

impl Slide {
    pub fn new(elements: Vec<Element>) -> Self {
        Self { elements }
    }
}
