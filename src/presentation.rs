use crate::{
    markdown::text::WeightedLine,
    render::{media::Image, properties::WindowSize},
    theme::{Alignment, Colors, PresentationTheme},
};
use serde::Deserialize;
use std::rc::Rc;

pub struct Presentation {
    pub slides: Vec<Slide>,
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

#[derive(Clone, Debug)]
pub struct Slide {
    pub render_operations: Vec<RenderOperation>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PresentationMetadata {
    pub title: Option<String>,

    #[serde(default)]
    pub sub_title: Option<String>,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub theme: PresentationThemeMetadata,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PresentationThemeMetadata {
    #[serde(default)]
    pub theme_name: Option<String>,

    #[serde(default)]
    pub theme_path: Option<String>,

    #[serde(default, rename = "override")]
    pub overrides: Option<PresentationTheme>,
}

#[derive(Clone, Debug)]
pub enum RenderOperation {
    ClearScreen,
    SetColors(Colors),
    JumpToVerticalCenter,
    JumpToSlideBottom,
    JumpToWindowBottom,
    RenderTextLine { texts: WeightedLine, alignment: Alignment },
    RenderSeparator,
    RenderLineBreak,
    RenderImage(Image),
    RenderPreformattedLine { text: String, unformatted_length: usize, block_length: usize, alignment: Alignment },
    RenderDynamic(Rc<dyn AsRenderOperations>),
}

pub trait AsRenderOperations: std::fmt::Debug {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation>;
}
