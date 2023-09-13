use crate::markdown::process::Slide;

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

    pub fn current_slide_index(&self) -> usize {
        self.current_slide_index
    }

    pub fn total_slides(&self) -> usize {
        self.slides.len()
    }

    pub fn jump_next_slide(&mut self) -> bool {
        if self.current_slide_index < self.slides.len() - 1 {
            self.current_slide_index += 1;
            true
        } else {
            false
        }
    }

    pub fn jump_previous_slide(&mut self) -> bool {
        if self.current_slide_index > 0 {
            self.current_slide_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn jump_first_slide(&mut self) -> bool {
        if self.current_slide_index != 0 {
            self.current_slide_index = 0;
            true
        } else {
            false
        }
    }

    pub fn jump_last_slide(&mut self) -> bool {
        let last_slide_index = self.slides.len().saturating_sub(1);
        if self.current_slide_index != last_slide_index {
            self.current_slide_index = last_slide_index;
            true
        } else {
            false
        }
    }

    pub fn jump_slide(&mut self, slide_index: usize) -> bool {
        if slide_index < self.slides.len() {
            self.current_slide_index = slide_index;
            true
        } else {
            false
        }
    }
}
