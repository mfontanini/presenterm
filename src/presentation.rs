use crate::{
    markdown::text::WeightedLine,
    render::media::Image,
    theme::{Alignment, Colors, PresentationTheme},
};
use serde::Deserialize;
use std::borrow::Cow;

pub struct Presentation<'a> {
    slides: Vec<Slide>,
    pub theme: Cow<'a, PresentationTheme>,
    current_slide_index: usize,
}

impl<'a> Presentation<'a> {
    pub fn new(slides: Vec<Slide>, theme: Cow<'a, PresentationTheme>) -> Self {
        Self { slides, theme, current_slide_index: 0 }
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

#[derive(Clone, Debug)]
pub struct Slide {
    pub render_operations: Vec<RenderOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PresentationMetadata {
    pub title: Option<String>,

    #[serde(default)]
    pub sub_title: Option<String>,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub theme: PresentationThemeMetadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PresentationThemeMetadata {
    #[serde(default)]
    pub theme_name: Option<String>,

    #[serde(default)]
    pub theme_path: Option<String>,
}

#[derive(Clone, Debug)]
pub enum RenderOperation {
    ClearScreen,
    SetColors(Colors),
    JumpToVerticalCenter,
    JumpToBottom,
    RenderTextLine { texts: WeightedLine, alignment: Alignment },
    RenderSeparator,
    RenderLineBreak,
    RenderImage(Image),
    RenderPreformattedLine { text: String, unformatted_length: usize, block_length: usize, alignment: Alignment },
}
