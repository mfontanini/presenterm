use crate::{config::OptionsConfig, render::operation::RenderOperation};
use serde::Deserialize;
use std::{
    cell::RefCell,
    fmt::Debug,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub(crate) mod builder;
pub(crate) mod diff;
pub(crate) mod poller;

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

    /// Iterate the slides in this presentation.
    pub(crate) fn iter_slides_mut(&mut self) -> impl Iterator<Item = &mut Slide> {
        self.slides.iter_mut()
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

    pub(crate) fn current_slide_mut(&mut self) -> &mut Slide {
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

    pub(crate) fn show_all_chunks(&mut self) {
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
    pub(crate) overrides: Option<crate::theme::raw::PresentationTheme>,
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
