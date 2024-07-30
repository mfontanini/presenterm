use crate::{
    custom::OptionsConfig,
    markdown::text::WeightedTextBlock,
    media::image::Image,
    render::properties::WindowSize,
    style::{Color, Colors},
    theme::{Alignment, Margin, PresentationTheme},
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) struct Modals {
    pub(crate) slide_index: Vec<RenderOperation>,
    pub(crate) bindings: Vec<RenderOperation>,
}

/// A presentation.
#[derive(Debug)]
pub(crate) struct Presentation {
    slides: Vec<Slide>,
    modals: Modals,
    pub(crate) state: PresentationState,
}

impl Presentation {
    /// Construct a new presentation.
    pub(crate) fn new(slides: Vec<Slide>, modals: Modals, state: PresentationState) -> Self {
        Self { slides, modals, state }
    }

    /// Iterate the slides in this presentation.
    pub(crate) fn iter_slides(&self) -> impl Iterator<Item = &Slide> {
        self.slides.iter()
    }

    /// Iterate the operations that render the slide index.
    pub(crate) fn iter_slide_index_operations(&self) -> impl Iterator<Item = &RenderOperation> {
        self.modals.slide_index.iter()
    }

    /// Iterate the operations that render the key bindings modal.
    pub(crate) fn iter_bindings_operations(&self) -> impl Iterator<Item = &RenderOperation> {
        self.modals.bindings.iter()
    }

    /// Consume this presentation and return its slides.
    #[cfg(test)]
    pub(crate) fn into_slides(self) -> Vec<Slide> {
        self.slides
    }

    /// Get the current slide.
    pub(crate) fn current_slide(&self) -> &Slide {
        &self.slides[self.current_slide_index()]
    }

    /// Get the current slide index.
    pub(crate) fn current_slide_index(&self) -> usize {
        self.state.current_slide_index()
    }

    /// Jump forwards.
    pub(crate) fn jump_next(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.move_next() {
            return true;
        }
        self.jump_next_slide()
    }

    /// Show all chunks in this slide, or jump to the next if already applied.
    pub(crate) fn jump_next_fast(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.visible_chunks == current_slide.chunks.len() {
            self.jump_next_slide()
        } else {
            current_slide.show_all_chunks();
            true
        }
    }

    /// Jump backwards.
    pub(crate) fn jump_previous(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.move_previous() {
            return true;
        }
        self.jump_previous_slide()
    }

    /// Show only the first chunk in this slide or jump to the previous slide if already there.
    pub(crate) fn jump_previous_fast(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.visible_chunks == current_slide.chunks.len() && current_slide.chunks.len() > 1 {
            current_slide.show_first_chunk();
            true
        } else {
            self.jump_previous_slide()
        }
    }

    /// Jump to the first slide.
    pub(crate) fn jump_first_slide(&mut self) -> bool {
        self.go_to_slide(0)
    }

    /// Jump to the last slide.
    pub(crate) fn jump_last_slide(&mut self) -> bool {
        let last_slide_index = self.slides.len().saturating_sub(1);
        self.go_to_slide(last_slide_index)
    }

    /// Jump to a specific slide.
    pub(crate) fn go_to_slide(&mut self, slide_index: usize) -> bool {
        if slide_index < self.slides.len() {
            self.state.set_current_slide_index(slide_index);
            // Always show only the first slide when jumping to a particular one.
            self.current_slide_mut().show_first_chunk();
            true
        } else {
            false
        }
    }

    /// Jump to a specific chunk within the current slide.
    pub(crate) fn jump_chunk(&mut self, chunk_index: usize) {
        self.current_slide_mut().jump_chunk(chunk_index);
    }

    /// Get the current slide's chunk.
    pub(crate) fn current_chunk(&self) -> usize {
        self.current_slide().current_chunk_index()
    }

    /// Trigger async render operations in all slides.
    pub(crate) fn trigger_all_async_renders(&mut self) -> HashSet<usize> {
        let mut triggered_slides = HashSet::new();
        for (index, slide) in self.slides.iter_mut().enumerate() {
            for operation in slide.iter_operations_mut() {
                if let RenderOperation::RenderAsync(operation) = operation {
                    if operation.start_render() {
                        triggered_slides.insert(index);
                    }
                }
            }
        }
        triggered_slides
    }

    /// Trigger async render operations in this slide.
    pub(crate) fn trigger_slide_async_renders(&mut self) -> bool {
        let slide = self.current_slide_mut();
        let mut any_rendered = false;
        for operation in slide.iter_visible_operations_mut() {
            if let RenderOperation::RenderAsync(operation) = operation {
                let is_rendered = operation.start_render();
                any_rendered = any_rendered || is_rendered;
            }
        }
        any_rendered
    }

    // Get all slides that contain async render operations.
    pub(crate) fn slides_with_async_renders(&self) -> HashSet<usize> {
        let mut indexes = HashSet::new();
        for (index, slide) in self.slides.iter().enumerate() {
            for operation in slide.iter_operations() {
                if let RenderOperation::RenderAsync(operation) = operation {
                    if matches!(operation.poll_state(), RenderAsyncState::Rendering { .. }) {
                        indexes.insert(index);
                        break;
                    }
                }
            }
        }
        indexes
    }

    /// Poll every async render operation in the current slide and check whether they're completed.
    pub(crate) fn poll_slide_async_renders(&mut self) -> RenderAsyncState {
        let slide = self.current_slide_mut();
        let mut slide_state = RenderAsyncState::Rendered;
        for operation in slide.iter_operations_mut() {
            if let RenderOperation::RenderAsync(operation) = operation {
                let state = operation.poll_state();
                slide_state = match (&slide_state, &state) {
                    // If one finished rendering and another one still is rendering, claim that we
                    // are still rendering and there's modifications.
                    (RenderAsyncState::JustFinishedRendering, RenderAsyncState::Rendering { .. })
                    | (RenderAsyncState::Rendering { .. }, RenderAsyncState::JustFinishedRendering) => {
                        RenderAsyncState::Rendering { modified: true }
                    }
                    // Render + modified overrides anything, rendering overrides only "rendered".
                    (_, RenderAsyncState::Rendering { modified: true })
                    | (RenderAsyncState::Rendered, RenderAsyncState::Rendering { .. })
                    | (_, RenderAsyncState::JustFinishedRendering) => state,
                    _ => slide_state,
                };
            }
        }
        slide_state
    }

    /// Run a callback through every operation and let it mutate it in place.
    ///
    /// This should be used with care!
    pub(crate) fn mutate_operations<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut RenderOperation),
    {
        for slide in &mut self.slides {
            for chunk in &mut slide.chunks {
                for operation in &mut chunk.operations {
                    callback(operation);
                }
            }
        }
    }

    fn current_slide_mut(&mut self) -> &mut Slide {
        let index = self.current_slide_index();
        &mut self.slides[index]
    }

    fn jump_next_slide(&mut self) -> bool {
        let current_slide_index = self.current_slide_index();
        if current_slide_index < self.slides.len() - 1 {
            self.state.set_current_slide_index(current_slide_index + 1);
            // Going forward we show only the first chunk.
            self.current_slide_mut().show_first_chunk();
            true
        } else {
            false
        }
    }

    fn jump_previous_slide(&mut self) -> bool {
        let current_slide_index = self.current_slide_index();
        if current_slide_index > 0 {
            self.state.set_current_slide_index(current_slide_index - 1);
            // Going backwards we show all chunks.
            self.current_slide_mut().show_all_chunks();
            true
        } else {
            false
        }
    }
}

impl From<Vec<Slide>> for Presentation {
    fn from(slides: Vec<Slide>) -> Self {
        let modals = Modals { slide_index: vec![], bindings: vec![] };
        Self::new(slides, modals, Default::default())
    }
}

#[derive(Debug)]
pub(crate) struct AsyncPresentationError {
    pub(crate) slide: usize,
    pub(crate) error: String,
}

pub(crate) type AsyncPresentationErrorHolder = Arc<Mutex<Option<AsyncPresentationError>>>;

#[derive(Debug, Default)]
pub(crate) struct PresentationStateInner {
    current_slide_index: usize,
    async_error_holder: AsyncPresentationErrorHolder,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PresentationState {
    inner: Rc<RefCell<PresentationStateInner>>,
}

impl PresentationState {
    pub(crate) fn async_error_holder(&self) -> AsyncPresentationErrorHolder {
        self.inner.deref().borrow().async_error_holder.clone()
    }

    pub(crate) fn current_slide_index(&self) -> usize {
        self.inner.deref().borrow().current_slide_index
    }

    fn set_current_slide_index(&self, value: usize) {
        self.inner.deref().borrow_mut().current_slide_index = value;
    }
}

/// A slide builder.
#[derive(Default)]
pub(crate) struct SlideBuilder {
    chunks: Vec<SlideChunk>,
    footer: Vec<RenderOperation>,
}

impl SlideBuilder {
    pub(crate) fn chunks(mut self, chunks: Vec<SlideChunk>) -> Self {
        self.chunks = chunks;
        self
    }

    pub(crate) fn footer(mut self, footer: Vec<RenderOperation>) -> Self {
        self.footer = footer;
        self
    }

    pub(crate) fn build(self) -> Slide {
        Slide::new(self.chunks, self.footer)
    }
}

/// A slide.
///
/// Slides are composed of render operations that can be carried out to materialize this slide into
/// the terminal's screen.
#[derive(Debug)]
pub(crate) struct Slide {
    chunks: Vec<SlideChunk>,
    footer: Vec<RenderOperation>,
    visible_chunks: usize,
}

impl Slide {
    pub(crate) fn new(chunks: Vec<SlideChunk>, footer: Vec<RenderOperation>) -> Self {
        Self { chunks, footer, visible_chunks: 1 }
    }

    pub(crate) fn iter_operations(&self) -> impl Iterator<Item = &RenderOperation> + Clone {
        self.chunks.iter().flat_map(|chunk| chunk.operations.iter()).chain(self.footer.iter())
    }

    pub(crate) fn iter_operations_mut(&mut self) -> impl Iterator<Item = &mut RenderOperation> {
        self.chunks.iter_mut().flat_map(|chunk| chunk.operations.iter_mut()).chain(self.footer.iter_mut())
    }

    pub(crate) fn iter_visible_operations(&self) -> impl Iterator<Item = &RenderOperation> + Clone {
        self.chunks.iter().take(self.visible_chunks).flat_map(|chunk| chunk.operations.iter()).chain(self.footer.iter())
    }

    pub(crate) fn iter_visible_operations_mut(&mut self) -> impl Iterator<Item = &mut RenderOperation> {
        self.chunks
            .iter_mut()
            .take(self.visible_chunks)
            .flat_map(|chunk| chunk.operations.iter_mut())
            .chain(self.footer.iter_mut())
    }

    pub(crate) fn iter_chunks(&self) -> impl Iterator<Item = &SlideChunk> {
        self.chunks.iter()
    }

    #[cfg(test)]
    pub(crate) fn into_operations(self) -> Vec<RenderOperation> {
        self.chunks.into_iter().flat_map(|chunk| chunk.operations.into_iter()).chain(self.footer).collect()
    }

    fn jump_chunk(&mut self, chunk_index: usize) {
        self.visible_chunks = chunk_index.saturating_add(1).min(self.chunks.len());
        for chunk in self.chunks.iter().take(self.visible_chunks - 1) {
            chunk.apply_all_mutations();
        }
    }

    fn current_chunk_index(&self) -> usize {
        self.visible_chunks.saturating_sub(1)
    }

    fn current_chunk(&self) -> &SlideChunk {
        &self.chunks[self.current_chunk_index()]
    }

    fn show_first_chunk(&mut self) {
        self.visible_chunks = 1;
        self.current_chunk().reset_mutations();
    }

    fn show_all_chunks(&mut self) {
        self.visible_chunks = self.chunks.len();
        for chunk in &self.chunks {
            chunk.apply_all_mutations();
        }
    }

    fn move_next(&mut self) -> bool {
        if self.chunks[self.current_chunk_index()].mutate_next() {
            return true;
        }

        if self.visible_chunks == self.chunks.len() {
            false
        } else {
            self.visible_chunks += 1;
            self.current_chunk().reset_mutations();
            true
        }
    }

    fn move_previous(&mut self) -> bool {
        if self.chunks[self.current_chunk_index()].mutate_previous() {
            return true;
        }
        if self.visible_chunks == 1 {
            false
        } else {
            self.visible_chunks -= 1;
            self.current_chunk().apply_all_mutations();
            true
        }
    }
}

impl From<Vec<RenderOperation>> for Slide {
    fn from(operations: Vec<RenderOperation>) -> Self {
        Self::new(vec![SlideChunk::new(operations, Vec::new())], vec![])
    }
}

#[derive(Debug, Default)]
pub(crate) struct SlideChunk {
    operations: Vec<RenderOperation>,
    mutators: Vec<Box<dyn ChunkMutator>>,
}

impl SlideChunk {
    pub(crate) fn new(operations: Vec<RenderOperation>, mutators: Vec<Box<dyn ChunkMutator>>) -> Self {
        Self { operations, mutators }
    }

    pub(crate) fn iter_operations(&self) -> impl Iterator<Item = &RenderOperation> + Clone {
        self.operations.iter()
    }

    pub(crate) fn pop_last(&mut self) -> Option<RenderOperation> {
        self.operations.pop()
    }

    fn mutate_next(&self) -> bool {
        for mutator in &self.mutators {
            if mutator.mutate_next() {
                return true;
            }
        }
        false
    }

    fn mutate_previous(&self) -> bool {
        for mutator in self.mutators.iter().rev() {
            if mutator.mutate_previous() {
                return true;
            }
        }
        false
    }

    fn reset_mutations(&self) {
        for mutator in &self.mutators {
            mutator.reset_mutations();
        }
    }

    fn apply_all_mutations(&self) {
        for mutator in &self.mutators {
            mutator.apply_all_mutations();
        }
    }
}

pub(crate) trait ChunkMutator: Debug {
    fn mutate_next(&self) -> bool;
    fn mutate_previous(&self) -> bool;
    fn reset_mutations(&self);
    fn apply_all_mutations(&self);
    #[allow(dead_code)]
    fn mutations(&self) -> (usize, usize);
}

/// The metadata for a presentation.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PresentationMetadata {
    /// The presentation title.
    pub(crate) title: Option<String>,

    /// The presentation sub-title.
    #[serde(default)]
    pub(crate) sub_title: Option<String>,

    /// The presentation event.
    #[serde(default)]
    pub(crate) event: Option<String>,

    /// The presentation location.
    #[serde(default)]
    pub(crate) location: Option<String>,

    /// The presentation date.
    #[serde(default)]
    pub(crate) date: Option<String>,

    /// The presentation author.
    #[serde(default)]
    pub(crate) author: Option<String>,

    /// The presentation authors.
    #[serde(default)]
    pub(crate) authors: Vec<String>,

    /// The presentation's theme metadata.
    #[serde(default)]
    pub(crate) theme: PresentationThemeMetadata,

    /// The presentation's options.
    #[serde(default)]
    pub(crate) options: Option<OptionsConfig>,
}

impl PresentationMetadata {
    /// Check if this presentation has frontmatter.
    pub(crate) fn has_frontmatter(&self) -> bool {
        self.title.is_some()
            || self.sub_title.is_some()
            || self.event.is_some()
            || self.location.is_some()
            || self.date.is_some()
            || self.author.is_some()
            || !self.authors.is_empty()
    }
}

/// A presentation's theme metadata.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct PresentationThemeMetadata {
    /// The theme name.
    #[serde(default)]
    pub(crate) name: Option<String>,

    /// the theme path.
    #[serde(default)]
    pub(crate) path: Option<String>,

    /// Any specific overrides for the presentation's theme.
    #[serde(default, rename = "override")]
    pub(crate) overrides: Option<PresentationTheme>,
}

/// A line of preformatted text to be rendered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BlockLine {
    pub(crate) text: BlockLineText,
    pub(crate) unformatted_length: u16,
    pub(crate) block_length: u16,
    pub(crate) alignment: Alignment,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockLineText {
    Preformatted(String),
    Weighted(WeightedTextBlock),
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

    /// Render text.
    RenderText { line: WeightedTextBlock, alignment: Alignment },

    /// Render a line break.
    RenderLineBreak,

    /// Render an image.
    RenderImage(Image, ImageProperties),

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
pub(crate) struct ImageProperties {
    pub(crate) z_index: i32,
    pub(crate) size: ImageSize,
    pub(crate) restore_cursor: bool,
    pub(crate) background_color: Option<Color>,
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
    pub(crate) horizontal_margin: Margin,

    /// The margin at the bottom of the slide.
    pub(crate) bottom_slide_margin: u16,
}

/// A type that can generate render operations.
pub(crate) trait AsRenderOperations: Debug + 'static {
    /// Generate render operations.
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation>;

    /// Get the content in this type to diff it against another `AsRenderOperations`.
    fn diffable_content(&self) -> Option<&str>;
}

/// An operation that can be rendered asynchronously.
pub(crate) trait RenderAsync: AsRenderOperations {
    /// Start the render for this operation.
    ///
    /// Should return true if the invocation triggered the rendering (aka if rendering wasn't
    /// already started before).
    fn start_render(&self) -> bool;

    /// Update the internal state and return the updated state.
    fn poll_state(&self) -> RenderAsyncState;
}

/// The state of a [RenderAsync].
#[derive(Clone, Debug, Default)]
pub(crate) enum RenderAsyncState {
    #[default]
    NotStarted,
    Rendering {
        modified: bool,
    },
    Rendered,
    JustFinishedRendering,
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;
    use std::cell::RefCell;

    #[derive(Clone)]
    enum Jump {
        First,
        Last,
        Next,
        NextFast,
        Previous,
        PreviousFast,
        Specific(usize),
    }

    impl Jump {
        fn apply(&self, presentation: &mut Presentation) {
            use Jump::*;
            match self {
                First => presentation.jump_first_slide(),
                Last => presentation.jump_last_slide(),
                Next => presentation.jump_next(),
                NextFast => presentation.jump_next_fast(),
                Previous => presentation.jump_previous(),
                PreviousFast => presentation.jump_previous_fast(),
                Specific(index) => presentation.go_to_slide(*index),
            };
        }

        fn repeat(&self, count: usize) -> Vec<Self> {
            vec![self.clone(); count]
        }
    }

    #[derive(Debug)]
    struct DummyMutator {
        current: RefCell<usize>,
        limit: usize,
    }

    impl DummyMutator {
        fn new(limit: usize) -> Self {
            Self { current: 0.into(), limit }
        }
    }

    impl ChunkMutator for DummyMutator {
        fn mutate_next(&self) -> bool {
            let mut current = self.current.borrow_mut();
            if *current < self.limit {
                *current += 1;
                true
            } else {
                false
            }
        }

        fn mutate_previous(&self) -> bool {
            let mut current = self.current.borrow_mut();
            if *current > 0 {
                *current -= 1;
                true
            } else {
                false
            }
        }

        fn reset_mutations(&self) {
            *self.current.borrow_mut() = 0;
        }

        fn apply_all_mutations(&self) {
            *self.current.borrow_mut() = self.limit;
        }

        fn mutations(&self) -> (usize, usize) {
            (*self.current.borrow(), self.limit)
        }
    }

    #[rstest]
    #[case::previous_from_first(0, &[Jump::Previous], 0, 0)]
    #[case::next_from_first(0, &[Jump::Next], 0, 1)]
    #[case::next_next_from_first(0, &[Jump::Next, Jump::Next], 0, 2)]
    #[case::next_next_next_from_first(0, &[Jump::Next, Jump::Next, Jump::Next], 1, 0)]
    #[case::next_fast_from_first(0, &[Jump::NextFast], 0, 2)]
    #[case::next_fast_twice_from_first(0, &[Jump::NextFast, Jump::NextFast], 1, 0)]
    #[case::last_from_first(0, &[Jump::Last], 2, 0)]
    #[case::previous_from_second(1, &[Jump::Previous], 0, 2)]
    #[case::previous_fast_from_second(1, &[Jump::PreviousFast], 0, 2)]
    #[case::previous_fast_twice_from_second(1, &[Jump::PreviousFast, Jump::PreviousFast], 0, 0)]
    #[case::next_from_second(1, &[Jump::Next], 1, 1)]
    #[case::specific_first_from_second(1, &[Jump::Specific(0)], 0, 0)]
    #[case::specific_last_from_second(1, &[Jump::Specific(2)], 2, 0)]
    #[case::first_from_last(2, &[Jump::First], 0, 0)]
    fn jumping(
        #[case] from: usize,
        #[case] jumps: &[Jump],
        #[case] expected_slide: usize,
        #[case] expected_chunk: usize,
    ) {
        let mut presentation = Presentation::from(vec![
            Slide::new(vec![SlideChunk::default(), SlideChunk::default(), SlideChunk::default()], vec![]),
            Slide::new(vec![SlideChunk::default(), SlideChunk::default()], vec![]),
            Slide::new(vec![SlideChunk::default(), SlideChunk::default()], vec![]),
        ]);
        presentation.go_to_slide(from);

        for jump in jumps {
            jump.apply(&mut presentation);
        }
        assert_eq!(presentation.current_slide_index(), expected_slide);
        assert_eq!(presentation.current_slide().visible_chunks - 1, expected_chunk);
    }

    #[rstest]
    #[case::next_1(0, &[Jump::Next], [1, 0, 0], 0, 0)]
    #[case::next_previous(0, &[Jump::Next, Jump::Previous], [0, 0, 0], 0, 0)]
    #[case::next_2(0, &Jump::Next.repeat(2), [1, 1, 0], 0, 0)]
    #[case::next_3(0, &Jump::Next.repeat(3), [1, 2, 0], 0, 0)]
    #[case::next_4(0, &Jump::Next.repeat(4), [1, 2, 0], 0, 1)]
    #[case::next_4_back_4(
        0,
        &[Jump::Next.repeat(4), Jump::Previous.repeat(4)].concat(),
        [0, 0, 0],
        0,
        0
    )]
    #[case::last_first(0, &[Jump::Last, Jump::First], [0, 0, 0], 0, 0)]
    #[case::back_from_second(0, &[Jump::Specific(1), Jump::Previous], [1, 2, 0], 0, 1)]
    #[case::specific_from_second(0, &[Jump::Specific(1), Jump::Previous, Jump::Specific(0)], [0, 0, 0], 0, 0)]
    fn jumping_with_mutations(
        #[case] from: usize,
        #[case] jumps: &[Jump],
        #[case] mutations: [usize; 3],
        #[case] expected_slide: usize,
        #[case] expected_chunk: usize,
    ) {
        let mut presentation = Presentation::from(vec![
            SlideBuilder::default()
                .chunks(vec![
                    SlideChunk::new(vec![], vec![Box::new(DummyMutator::new(1)), Box::new(DummyMutator::new(2))]),
                    SlideChunk::default(),
                ])
                .build(),
            SlideBuilder::default()
                .chunks(vec![SlideChunk::new(vec![], vec![Box::new(DummyMutator::new(2))]), SlideChunk::default()])
                .build(),
        ]);
        presentation.go_to_slide(from);

        for jump in jumps {
            jump.apply(&mut presentation);
        }
        let mutators: Vec<_> = presentation
            .iter_slides()
            .flat_map(|slide| slide.chunks.iter())
            .flat_map(|chunk| chunk.mutators.iter())
            .collect();
        assert_eq!(mutators.len(), mutations.len(), "unexpected mutation count");
        for (index, (mutator, expected_mutations)) in mutators.into_iter().zip(mutations).enumerate() {
            assert_eq!(mutator.mutations().0, expected_mutations, "diff on {index}");
        }
        assert_eq!(presentation.current_slide_index(), expected_slide, "slide differs");
        assert_eq!(presentation.current_slide().visible_chunks - 1, expected_chunk, "chunk differs");
    }
}
