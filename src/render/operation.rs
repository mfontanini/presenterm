use super::properties::WindowSize;
use crate::{
    markdown::{
        text::{WeightedLine, WeightedText},
        text_style::{Color, Colors},
    },
    terminal::image::Image,
    theme::{Alignment, Margin},
};
use std::{
    fmt::Debug,
    rc::Rc,
    sync::{Arc, Mutex},
};

const DEFAULT_IMAGE_Z_INDEX: i32 = -2;

/// A line of preformatted text to be rendered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BlockLine {
    pub(crate) prefix: WeightedText,
    pub(crate) right_padding_length: u16,
    pub(crate) repeat_prefix_on_wrap: bool,
    pub(crate) text: WeightedLine,
    pub(crate) block_length: u16,
    pub(crate) block_color: Option<Color>,
    pub(crate) alignment: Alignment,
}

/// A render operation.
///
/// Render operations are primitives that allow the input markdown file to be decoupled with what
/// we draw on the screen.
#[derive(Clone, Debug)]
pub(crate) enum RenderOperation {
    /// Clear the entire screen.
    ClearScreen,

    /// Set the colors to be used for any subsequent operations.
    SetColors(Colors),

    /// Jump the draw cursor into the vertical center, that is, at `screen_height / 2`.
    JumpToVerticalCenter,

    /// Jumps to the N-th row in the current layout.
    ///
    /// The index is zero based where 0 represents the top row.
    JumpToRow { index: u16 },

    /// Jumps to the N-th to last row in the current layout.
    ///
    /// The index is zero based where 0 represents the bottom row.
    JumpToBottomRow { index: u16 },

    /// Jump to the N-th column in the current layout.
    JumpToColumn { index: u16 },

    /// Render text.
    RenderText { line: WeightedLine, alignment: Alignment },

    /// Render a line break.
    RenderLineBreak,

    /// Render an image.
    RenderImage(Image, ImageRenderProperties),

    /// Render a line.
    RenderBlockLine(BlockLine),

    /// Render a dynamically generated sequence of render operations.
    ///
    /// This allows drawing something on the screen that requires knowing dynamic properties of the
    /// screen, like window size, without coupling the transformation of markdown into
    /// [RenderOperation] with the screen itself.
    RenderDynamic(Rc<dyn AsRenderOperations>),

    /// An operation that is rendered asynchronously.
    RenderAsync(Rc<dyn RenderAsync>),

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

    /// Apply a margin to every following operation.
    ApplyMargin(MarginProperties),

    /// Pop an `ApplyMargin` operation.
    PopMargin,
}

/// The properties of an image being rendered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImageRenderProperties {
    pub(crate) z_index: i32,
    pub(crate) size: ImageSize,
    pub(crate) restore_cursor: bool,
    pub(crate) background_color: Option<Color>,
    pub(crate) position: ImagePosition,
}

impl Default for ImageRenderProperties {
    fn default() -> Self {
        Self {
            z_index: DEFAULT_IMAGE_Z_INDEX,
            size: Default::default(),
            restore_cursor: false,
            background_color: None,
            position: ImagePosition::Center,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImagePosition {
    Cursor,
    Center,
    Right,
}

/// The size used when printing an image.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum ImageSize {
    #[default]
    ShrinkIfNeeded,
    Specific(u16, u16),
    WidthScaled {
        ratio: f64,
    },
}

/// Slide properties, set on initialization.
#[derive(Clone, Debug, Default)]
pub(crate) struct MarginProperties {
    /// The horizontal margin.
    pub(crate) horizontal: Margin,

    /// The margin at the top.
    pub(crate) top: u16,

    /// The margin at the bottom.
    pub(crate) bottom: u16,
}

/// A type that can generate render operations.
pub(crate) trait AsRenderOperations: Debug + 'static {
    /// Generate render operations.
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation>;

    /// Get the content in this type to diff it against another `AsRenderOperations`.
    fn diffable_content(&self) -> Option<&str> {
        None
    }
}

/// An operation that can be rendered asynchronously.
pub(crate) trait RenderAsync: AsRenderOperations {
    /// Create a pollable for this render async.
    ///
    /// The pollable will be used to poll this by a separate thread, so all state that will
    /// be loaded asynchronously should be shared between this operation and any pollables
    /// generated from it.
    fn pollable(&self) -> Box<dyn Pollable>;

    /// Get the start policy for this render.
    fn start_policy(&self) -> RenderAsyncStartPolicy {
        RenderAsyncStartPolicy::OnDemand
    }
}

/// The start policy for an async render.
#[derive(Copy, Clone, Debug)]
pub(crate) enum RenderAsyncStartPolicy {
    /// Start automatically.
    Automatic,

    /// Start on demand.
    OnDemand,
}

/// A pollable that can be used to pull and update the state of an operation asynchronously.
pub(crate) trait Pollable: Send + 'static {
    /// Update the internal state and return the updated state.
    fn poll(&mut self) -> PollableState;
}

/// The state of a [Pollable].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PollableState {
    Unmodified,
    Modified,
    Done,
    Failed { error: String },
}

pub(crate) struct ToggleState {
    toggled: Arc<Mutex<bool>>,
}

impl ToggleState {
    pub(crate) fn new(toggled: Arc<Mutex<bool>>) -> Self {
        Self { toggled }
    }
}

impl Pollable for ToggleState {
    fn poll(&mut self) -> PollableState {
        *self.toggled.lock().unwrap() = true;
        PollableState::Done
    }
}
