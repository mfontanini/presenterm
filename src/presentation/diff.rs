use crate::presentation::{Presentation, RenderOperation, SlideChunk};
use std::{any::Any, cmp::Ordering, fmt::Debug, mem};

/// Allow diffing presentations.
pub(crate) struct PresentationDiffer;

impl PresentationDiffer {
    /// Find the first modification between two presentations.
    pub(crate) fn find_first_modification(original: &Presentation, updated: &Presentation) -> Option<Modification> {
        let original_slides = original.iter_slides();
        let updated_slides = updated.iter_slides();
        for (slide_index, (original, updated)) in original_slides.zip(updated_slides).enumerate() {
            for (chunk_index, (original, updated)) in original.iter_chunks().zip(updated.iter_chunks()).enumerate() {
                if original.is_content_different(updated) {
                    return Some(Modification { slide_index, chunk_index });
                }
            }
            let total_original = original.iter_chunks().count();
            let total_updated = updated.iter_chunks().count();
            match total_original.cmp(&total_updated) {
                Ordering::Equal => (),
                Ordering::Less => return Some(Modification { slide_index, chunk_index: total_original }),
                Ordering::Greater => {
                    return Some(Modification { slide_index, chunk_index: total_updated.saturating_sub(1) });
                }
            }
        }
        let total_original = original.iter_slides().count();
        let total_updated = updated.iter_slides().count();
        match total_original.cmp(&total_updated) {
            // If they have the same number of slides there's no difference.
            Ordering::Equal => None,
            // If the original had fewer, let's scroll to the first new one.
            Ordering::Less => Some(Modification { slide_index: total_original, chunk_index: 0 }),
            // If the original had more, let's scroll to the last one.
            Ordering::Greater => {
                Some(Modification { slide_index: total_updated.saturating_sub(1), chunk_index: usize::MAX })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Modification {
    pub(crate) slide_index: usize,
    pub(crate) chunk_index: usize,
}

trait ContentDiff {
    fn is_content_different(&self, other: &Self) -> bool;
}

impl ContentDiff for SlideChunk {
    fn is_content_different(&self, other: &Self) -> bool {
        self.iter_operations().is_content_different(&other.iter_operations())
    }
}

impl ContentDiff for RenderOperation {
    fn is_content_different(&self, other: &Self) -> bool {
        use RenderOperation::*;
        let same_variant = mem::discriminant(self) == mem::discriminant(other);
        // If variants don't even match, content is different.
        if !same_variant {
            return true;
        }

        match (self, other) {
            (SetColors(original), SetColors(updated)) if original != updated => false,
            (RenderText { line: original, .. }, RenderText { line: updated, .. }) if original != updated => true,
            (RenderText { alignment: original, .. }, RenderText { alignment: updated, .. }) if original != updated => {
                false
            }
            (RenderImage(original, original_properties), RenderImage(updated, updated_properties))
                if original != updated || original_properties != updated_properties =>
            {
                true
            }
            (RenderBlockLine(original), RenderBlockLine(updated)) if original != updated => true,
            (InitColumnLayout { columns: original, .. }, InitColumnLayout { columns: updated, .. })
                if original != updated =>
            {
                true
            }
            (EnterColumn { column: original }, EnterColumn { column: updated }) if original != updated => true,
            (RenderDynamic(original), RenderDynamic(updated)) if original.type_id() != updated.type_id() => true,
            (RenderDynamic(original), RenderDynamic(updated)) => {
                original.diffable_content() != updated.diffable_content()
            }
            (RenderAsync(original), RenderAsync(updated)) if original.type_id() != updated.type_id() => true,
            (RenderAsync(original), RenderAsync(updated)) => original.diffable_content() != updated.diffable_content(),
            _ => false,
        }
    }
}

impl<'a, T, U> ContentDiff for T
where
    T: IntoIterator<Item = &'a U> + Clone,
    U: ContentDiff + 'a,
{
    fn is_content_different(&self, other: &Self) -> bool {
        let lhs = self.clone().into_iter();
        let rhs = other.clone().into_iter();
        for (lhs, rhs) in lhs.zip(rhs) {
            if lhs.is_content_different(rhs) {
                return true;
            }
        }
        // If either have more than the other, they've changed
        self.clone().into_iter().count() != other.clone().into_iter().count()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        markdown::{
            text::WeightedLine,
            text_style::{Color, Colors},
        },
        presentation::{Slide, SlideBuilder},
        render::{
            operation::{AsRenderOperations, BlockLine, LayoutGrid, Pollable, RenderAsync, ToggleState},
            properties::WindowSize,
        },
        theme::{Alignment, Margin},
    };
    use rstest::rstest;
    use std::rc::Rc;

    #[derive(Debug)]
    struct Dynamic;

    impl AsRenderOperations for Dynamic {
        fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
            Vec::new()
        }
    }

    impl RenderAsync for Dynamic {
        fn pollable(&self) -> Box<dyn Pollable> {
            // Use some random one, we don't care
            Box::new(ToggleState::new(Default::default()))
        }
    }

    #[rstest]
    #[case(RenderOperation::ClearScreen)]
    #[case(RenderOperation::JumpToVerticalCenter)]
    #[case(RenderOperation::JumpToBottomRow{ index: 0 })]
    #[case(RenderOperation::RenderLineBreak)]
    #[case(RenderOperation::SetColors(Colors{background: None, foreground: None}))]
    #[case(RenderOperation::RenderText{line: String::from("asd").into(), alignment: Default::default()})]
    #[case(RenderOperation::RenderBlockLine(
        BlockLine{
            prefix: "".into(),
            right_padding_length: 0,
            repeat_prefix_on_wrap: false,
            text: WeightedLine::from("".to_string()),
            alignment: Default::default(),
            block_length: 42,
            block_color: None,
        }
    ))]
    #[case(RenderOperation::RenderDynamic(Rc::new(Dynamic)))]
    #[case(RenderOperation::RenderAsync(Rc::new(Dynamic)))]
    #[case(RenderOperation::InitColumnLayout{ columns: vec![1, 2], grid: LayoutGrid::None })]
    #[case(RenderOperation::EnterColumn{ column: 1 })]
    #[case(RenderOperation::ExitLayout)]
    fn same_not_modified(#[case] operation: RenderOperation) {
        let diff = operation.is_content_different(&operation);
        assert!(!diff);
    }

    #[test]
    fn different_text() {
        let lhs = RenderOperation::RenderText { line: String::from("foo").into(), alignment: Default::default() };
        let rhs = RenderOperation::RenderText { line: String::from("bar").into(), alignment: Default::default() };
        assert!(lhs.is_content_different(&rhs));
    }

    #[test]
    fn different_text_alignment() {
        let lhs = RenderOperation::RenderText {
            line: String::from("foo").into(),
            alignment: Alignment::Left { margin: Margin::Fixed(42) },
        };
        let rhs = RenderOperation::RenderText {
            line: String::from("foo").into(),
            alignment: Alignment::Left { margin: Margin::Fixed(1337) },
        };
        assert!(!lhs.is_content_different(&rhs));
    }

    #[test]
    fn different_colors() {
        let lhs = RenderOperation::SetColors(Colors { background: None, foreground: Some(Color::new(1, 2, 3)) });
        let rhs = RenderOperation::SetColors(Colors { background: None, foreground: Some(Color::new(3, 2, 1)) });
        assert!(!lhs.is_content_different(&rhs));
    }

    #[test]
    fn different_column_layout() {
        let lhs = RenderOperation::InitColumnLayout { columns: vec![1, 2], grid: LayoutGrid::None };
        let rhs = RenderOperation::InitColumnLayout { columns: vec![1, 3], grid: LayoutGrid::None };
        assert!(lhs.is_content_different(&rhs));
    }

    #[test]
    fn different_column() {
        let lhs = RenderOperation::EnterColumn { column: 0 };
        let rhs = RenderOperation::EnterColumn { column: 1 };
        assert!(lhs.is_content_different(&rhs));
    }

    #[test]
    fn no_slide_changes() {
        let presentation = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
        ]);
        assert_eq!(PresentationDiffer::find_first_modification(&presentation, &presentation), None);
    }

    #[test]
    fn slides_truncated() {
        let lhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
        ]);
        let rhs = Presentation::from(vec![Slide::from(vec![RenderOperation::ClearScreen])]);

        assert_eq!(
            PresentationDiffer::find_first_modification(&lhs, &rhs),
            Some(Modification { slide_index: 0, chunk_index: usize::MAX })
        );
    }

    #[test]
    fn slides_added() {
        let lhs = Presentation::from(vec![Slide::from(vec![RenderOperation::ClearScreen])]);
        let rhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
        ]);

        assert_eq!(
            PresentationDiffer::find_first_modification(&lhs, &rhs),
            Some(Modification { slide_index: 1, chunk_index: 0 })
        );
    }

    #[test]
    fn second_slide_content_changed() {
        let lhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::ClearScreen]),
        ]);
        let rhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            Slide::from(vec![RenderOperation::JumpToVerticalCenter]),
            Slide::from(vec![RenderOperation::ClearScreen]),
        ]);

        assert_eq!(
            PresentationDiffer::find_first_modification(&lhs, &rhs),
            Some(Modification { slide_index: 1, chunk_index: 0 })
        );
    }

    #[test]
    fn presentation_changed_style() {
        let lhs = Presentation::from(vec![Slide::from(vec![RenderOperation::SetColors(Colors {
            background: None,
            foreground: Some(Color::new(255, 0, 0)),
        })])]);
        let rhs = Presentation::from(vec![Slide::from(vec![RenderOperation::SetColors(Colors {
            background: None,
            foreground: Some(Color::new(0, 0, 0)),
        })])]);

        assert_eq!(PresentationDiffer::find_first_modification(&lhs, &rhs), None);
    }

    #[test]
    fn chunk_change() {
        let lhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            SlideBuilder::default()
                .chunks(vec![SlideChunk::default(), SlideChunk::new(vec![RenderOperation::ClearScreen], vec![])])
                .build(),
        ]);
        let rhs = Presentation::from(vec![
            Slide::from(vec![RenderOperation::ClearScreen]),
            SlideBuilder::default()
                .chunks(vec![
                    SlideChunk::default(),
                    SlideChunk::new(vec![RenderOperation::ClearScreen, RenderOperation::ClearScreen], vec![]),
                ])
                .build(),
        ]);

        assert_eq!(
            PresentationDiffer::find_first_modification(&lhs, &rhs),
            Some(Modification { slide_index: 1, chunk_index: 1 })
        );
        assert_eq!(
            PresentationDiffer::find_first_modification(&rhs, &lhs),
            Some(Modification { slide_index: 1, chunk_index: 1 })
        );
    }
}
