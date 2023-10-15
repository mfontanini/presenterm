use crate::{
    markdown::text::WeightedLine,
    render::{media::Image, properties::WindowSize},
    style::Colors,
    theme::{Alignment, PresentationTheme},
};
use serde::Deserialize;
use std::rc::Rc;

/// A presentation.
pub struct Presentation {
    slides: Vec<Slide>,
    current_slide_index: usize,
}

impl Presentation {
    /// Construct a new presentation.
    pub fn new(slides: Vec<Slide>) -> Self {
        Self { slides, current_slide_index: 0 }
    }

    /// Iterate the slides in this presentation.
    pub fn iter_slides(&self) -> impl Iterator<Item = &Slide> {
        self.slides.iter()
    }

    /// Consume this presentation and return its slides.
    pub fn into_slides(self) -> Vec<Slide> {
        self.slides
    }

    /// Get the current slide.
    pub fn current_slide(&self) -> &Slide {
        &self.slides[self.current_slide_index]
    }

    /// Get the current slide index.
    pub fn current_slide_index(&self) -> usize {
        self.current_slide_index
    }

    /// Jump to the next slide.
    pub fn jump_next_slide(&mut self) -> bool {
        if self.current_slide_index < self.slides.len() - 1 {
            self.current_slide_index += 1;
            true
        } else {
            false
        }
    }

    /// Jump to the previous slide.
    pub fn jump_previous_slide(&mut self) -> bool {
        if self.current_slide_index > 0 {
            self.current_slide_index -= 1;
            true
        } else {
            false
        }
    }

    /// Jump to the first slide.
    pub fn jump_first_slide(&mut self) -> bool {
        if self.current_slide_index != 0 {
            self.current_slide_index = 0;
            true
        } else {
            false
        }
    }

    /// Jump to the last slide.
    pub fn jump_last_slide(&mut self) -> bool {
        let last_slide_index = self.slides.len().saturating_sub(1);
        if self.current_slide_index != last_slide_index {
            self.current_slide_index = last_slide_index;
            true
        } else {
            false
        }
    }

    /// Jump to a specific slide.
    pub fn jump_slide(&mut self, slide_index: usize) -> bool {
        if slide_index < self.slides.len() {
            self.current_slide_index = slide_index;
            true
        } else {
            false
        }
    }
}

/// A slide.
///
/// Slides are composed of render operations that can be carried out to materialize this slide into
/// the terminal's screen.
#[derive(Clone, Debug)]
pub struct Slide {
    pub render_operations: Vec<RenderOperation>,
}

/// The metadata for a presentation.
#[derive(Clone, Debug, Deserialize)]
pub struct PresentationMetadata {
    /// The presentation title.
    pub title: Option<String>,

    /// The presentation sub-title.
    #[serde(default)]
    pub sub_title: Option<String>,

    /// The presentation author.
    #[serde(default)]
    pub author: Option<String>,

    /// The presentation's theme metadata.
    #[serde(default)]
    pub theme: PresentationThemeMetadata,
}

/// A presentation's theme metadata.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PresentationThemeMetadata {
    /// The theme name.
    #[serde(default)]
    pub name: Option<String>,

    /// the theme path.
    #[serde(default)]
    pub path: Option<String>,

    /// Any specific overrides for the presentation's theme.
    #[serde(default, rename = "override")]
    pub overrides: Option<PresentationTheme>,
}

/// A line of preformatted text to be rendered.
#[derive(Clone, Debug, PartialEq)]
pub struct PreformattedLine {
    pub text: String,
    pub unformatted_length: usize,
    pub block_length: usize,
    pub alignment: Alignment,
}

/// A render operation.
///
/// Render operations are primitives that allow the input markdown file to be decoupled with what
/// we draw on the screen.
#[derive(Clone, Debug)]
pub enum RenderOperation {
    /// Clear the entire screen.
    ClearScreen,

    /// Set the colors to be used for any subsequent operations.
    SetColors(Colors),

    /// Jump the draw cursor into the vertical center, that is, at `screen_height / 2`.
    JumpToVerticalCenter,

    /// Jump the draw cursor into the last row in the screen.
    JumpToWindowBottom,

    /// Jumps to the last row in the slide.
    ///
    /// The slide's draw area is slightly smaller than the window, hence the distinction with
    /// [RenderOperation::JumpToWindowBottom].
    JumpToSlideBottom,

    /// Render a line of text.
    RenderTextLine { line: WeightedLine, alignment: Alignment },

    /// Render a horizontal separator line.
    RenderSeparator,

    /// Render a line break.
    RenderLineBreak,

    /// Render an image.
    RenderImage(Image),

    /// Render a preformatted line.
    ///
    /// The line will usually already have terminal escape codes that include colors and formatting
    /// embedded in it.
    RenderPreformattedLine(PreformattedLine),

    /// Render a dynamically generated sequence of render operations.
    ///
    /// This allows drawing something on the screen that requires knowing dynamic properties of the
    /// screen, like window size, without coupling the transformation of markdown into
    /// [RenderOperation] with the screen itself.
    RenderDynamic(Rc<dyn AsRenderOperations>),

    /// Initialize a column layout.
    ///
    /// The value for each column is the width of the column in column-unit units, where the entire
    /// screen contains `columns.sum()` column-units.
    InitColumnLayout { columns: Vec<u8> },

    /// Enter a column in a column layout.
    ///
    /// The index is 0-index based and will be tied to a previous `InitColumnLayout` operation.
    EnterColumn { column: usize },

    /// Exit the current layout and go back to the default one.
    ExitLayout,
}

/// A type that can generate render operations.
pub trait AsRenderOperations: std::fmt::Debug {
    /// Generate render operations.
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation>;
}
