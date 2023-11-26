use crate::{
    markdown::text::WeightedLine,
    render::{media::Image, properties::WindowSize},
    style::Colors,
    theme::{Alignment, Margin, PresentationTheme},
};
use serde::Deserialize;
use std::{fmt::Debug, rc::Rc};

/// A presentation.
#[derive(Debug)]
pub(crate) struct Presentation {
    slides: Vec<Slide>,
    current_slide_index: usize,
}

impl Presentation {
    /// Construct a new presentation.
    pub(crate) fn new(slides: Vec<Slide>) -> Self {
        Self { slides, current_slide_index: 0 }
    }

    /// Iterate the slides in this presentation.
    pub(crate) fn iter_slides(&self) -> impl Iterator<Item = &Slide> {
        self.slides.iter()
    }

    /// Consume this presentation and return its slides.
    #[cfg(test)]
    pub(crate) fn into_slides(self) -> Vec<Slide> {
        self.slides
    }

    /// Get the current slide.
    pub(crate) fn current_slide(&self) -> &Slide {
        &self.slides[self.current_slide_index]
    }

    /// Get the current slide index.
    pub(crate) fn current_slide_index(&self) -> usize {
        self.current_slide_index
    }

    /// Jump to the next slide.
    pub(crate) fn jump_next_slide(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.move_next() {
            return true;
        }
        if self.current_slide_index < self.slides.len() - 1 {
            self.current_slide_index += 1;
            // Going forward we show only the first chunk.
            self.current_slide_mut().show_first_chunk();
            true
        } else {
            false
        }
    }

    /// Jump to the previous slide.
    pub(crate) fn jump_previous_slide(&mut self) -> bool {
        let current_slide = self.current_slide_mut();
        if current_slide.move_previous() {
            return true;
        }
        if self.current_slide_index > 0 {
            self.current_slide_index -= 1;
            // Going backwards we show all chunks.
            self.current_slide_mut().show_all_chunks();
            true
        } else {
            false
        }
    }

    /// Jump to the first slide.
    pub(crate) fn jump_first_slide(&mut self) -> bool {
        self.jump_slide(0)
    }

    /// Jump to the last slide.
    pub(crate) fn jump_last_slide(&mut self) -> bool {
        let last_slide_index = self.slides.len().saturating_sub(1);
        self.jump_slide(last_slide_index)
    }

    /// Jump to a specific slide.
    pub(crate) fn jump_slide(&mut self, slide_index: usize) -> bool {
        if slide_index < self.slides.len() {
            self.current_slide_index = slide_index;
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

    /// Render all widgets in this slide.
    pub(crate) fn render_slide_widgets(&mut self) -> bool {
        let slide = self.current_slide_mut();
        let mut any_rendered = false;
        for operation in slide.iter_operations_mut() {
            if let RenderOperation::RenderOnDemand(operation) = operation {
                any_rendered = any_rendered || operation.start_render();
            }
        }
        any_rendered
    }

    /// Poll every widget in the current slide and check whether they're rendered.
    pub(crate) fn widgets_rendered(&mut self) -> bool {
        let slide = self.current_slide_mut();
        let mut all_rendered = true;
        for operation in slide.iter_operations_mut() {
            if let RenderOperation::RenderOnDemand(operation) = operation {
                all_rendered = all_rendered && matches!(operation.poll_state(), RenderOnDemandState::Rendered);
            }
        }
        all_rendered
    }

    fn current_slide_mut(&mut self) -> &mut Slide {
        &mut self.slides[self.current_slide_index]
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
        self.chunks.iter().take(self.visible_chunks).flat_map(|chunk| chunk.operations.iter()).chain(self.footer.iter())
    }

    pub(crate) fn iter_operations_mut(&mut self) -> impl Iterator<Item = &mut RenderOperation> {
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
        self.chunks.into_iter().flat_map(|chunk| chunk.operations.into_iter()).chain(self.footer.into_iter()).collect()
    }

    fn jump_chunk(&mut self, chunk_index: usize) {
        self.visible_chunks = (chunk_index + 1).min(self.chunks.len());
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

    /// The presentation author.
    #[serde(default)]
    pub(crate) author: Option<String>,

    /// The presentation's theme metadata.
    #[serde(default)]
    pub(crate) theme: PresentationThemeMetadata,
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
pub(crate) struct PreformattedLine {
    pub(crate) text: String,
    pub(crate) unformatted_length: usize,
    pub(crate) block_length: usize,
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

    /// Jumps to the N-th to last row in the slide.
    ///
    /// The index is zero based where 0 represents the bottom row.
    JumpToBottomRow { index: u16 },

    /// Render text.
    RenderText { line: WeightedLine, alignment: Alignment },

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

    /// An operation that is rendered on demand.
    RenderOnDemand(Rc<dyn RenderOnDemand>),

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

/// A type that can be rendered on demand.
pub(crate) trait RenderOnDemand: AsRenderOperations {
    /// Start the on demand render for this operation.
    fn start_render(&self) -> bool;

    /// Poll and update the internal on demand state and return the latest.
    fn poll_state(&self) -> RenderOnDemandState;
}

/// The state of a [RenderOnDemand].
#[derive(Clone, Debug, Default)]
pub(crate) enum RenderOnDemandState {
    #[default]
    NotStarted,
    Rendering,
    Rendered,
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;
    use rstest::rstest;

    #[derive(Clone)]
    enum Jump {
        First,
        Last,
        Next,
        Previous,
        Specific(usize),
    }

    impl Jump {
        fn apply(&self, presentation: &mut Presentation) {
            use Jump::*;
            match self {
                First => presentation.jump_first_slide(),
                Last => presentation.jump_last_slide(),
                Next => presentation.jump_next_slide(),
                Previous => presentation.jump_previous_slide(),
                Specific(index) => presentation.jump_slide(*index),
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
    #[case::next_next_from_first(0, &[Jump::Next, Jump::Next], 1, 0)]
    #[case::last_from_first(0, &[Jump::Last], 2, 0)]
    #[case::previous_from_second(1, &[Jump::Previous], 0, 1)]
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
        let mut presentation = Presentation::new(vec![
            Slide::new(vec![SlideChunk::from(SlideChunk::default()), SlideChunk::default()], vec![]),
            Slide::new(vec![SlideChunk::from(SlideChunk::default()), SlideChunk::default()], vec![]),
            Slide::new(vec![SlideChunk::from(SlideChunk::default()), SlideChunk::default()], vec![]),
        ]);
        presentation.jump_slide(from);

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
        let mut presentation = Presentation::new(vec![
            Slide::new(
                vec![
                    SlideChunk::from(SlideChunk::new(
                        vec![],
                        vec![Box::new(DummyMutator::new(1)), Box::new(DummyMutator::new(2))],
                    )),
                    SlideChunk::default(),
                ],
                vec![],
            ),
            Slide::new(
                vec![
                    SlideChunk::from(SlideChunk::new(vec![], vec![Box::new(DummyMutator::new(2))])),
                    SlideChunk::default(),
                ],
                vec![],
            ),
        ]);
        presentation.jump_slide(from);

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
