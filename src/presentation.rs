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

pub struct Presentation {
    slides: Vec<Slide>,
    current_slide_index: usize,
}

impl Presentation {
    pub fn new(slides: Vec<Slide>) -> Self {
        Self { slides, current_slide_index: 0 }
    }

    pub fn current_slide(&self) -> &Slide {
        &self.slides[self.current_slide_index]
    }

    pub fn move_next_slide(&mut self) {
        if self.current_slide_index < self.slides.len() - 1 {
            self.current_slide_index += 1;
        }
    }

    pub fn move_previous_slide(&mut self) {
        if self.current_slide_index > 0 {
            self.current_slide_index -= 1;
        }
    }
}
