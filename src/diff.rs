use crate::presentation::{Presentation, RenderOperation, Slide};
use std::{cmp::Ordering, mem};

/// Allow diffing presentations.
pub struct PresentationDiffer;

impl PresentationDiffer {
    /// Find the first modified slide between original and updated.
    ///
    /// This tries to take into account both content and style changes such that changing
    pub fn first_modified_slide(original: &Presentation, updated: &Presentation) -> Option<usize> {
        let original_slides = original.iter_slides();
        let updated_slides = updated.iter_slides();
        for (index, (original, updated)) in original_slides.zip(updated_slides).enumerate() {
            if original.is_content_different(updated) {
                return Some(index);
            }
        }
        let total_original = original.iter_slides().count();
        let total_updated = updated.iter_slides().count();
        match total_original.cmp(&total_updated) {
            // If they have the same number of slides there's no difference.
            Ordering::Equal => None,
            // If the original had fewer, let's scroll to the first new one.
            Ordering::Less => Some(total_original),
            // If the original had more, let's scroll to the last one.
            Ordering::Greater => Some(total_updated.saturating_sub(1)),
        }
    }
}

trait ContentDiff {
    fn is_content_different(&self, other: &Self) -> bool;
}

impl ContentDiff for Slide {
    fn is_content_different(&self, other: &Self) -> bool {
        self.render_operations.iter().is_content_different(&other.render_operations.iter())
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
            (RenderTextLine { line: original, .. }, RenderTextLine { line: updated, .. }) if original != updated => {
                true
            }
            (RenderTextLine { alignment: original, .. }, RenderTextLine { alignment: updated, .. })
                if original != updated =>
            {
                false
            }
            (RenderImage(original), RenderImage(updated)) if original != updated => true,
            (RenderPreformattedLine(original), RenderPreformattedLine(updated)) if original != updated => true,
            // This is only used for footers which are global. Ignore for now.
            (RenderDynamic(_), RenderDynamic(_)) => false,
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
        let mut lhs = self.clone().into_iter();
        let mut rhs = other.clone().into_iter();
        for (lhs, rhs) in lhs.by_ref().zip(rhs.by_ref()) {
            if lhs.is_content_different(rhs) {
                return true;
            }
        }
        // If either have more than the other, they've changed
        lhs.next().is_some() != rhs.next().is_some()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        presentation::{AsRenderOperations, PreformattedLine},
        render::properties::WindowSize,
        style::{Color, Colors},
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

    #[rstest]
    #[case(RenderOperation::ClearScreen)]
    #[case(RenderOperation::JumpToVerticalCenter)]
    #[case(RenderOperation::JumpToSlideBottom)]
    #[case(RenderOperation::JumpToWindowBottom)]
    #[case(RenderOperation::RenderSeparator)]
    #[case(RenderOperation::RenderLineBreak)]
    #[case(RenderOperation::SetColors(Colors{background: None, foreground: None}))]
    #[case(RenderOperation::RenderTextLine{line: String::from("asd").into(), alignment: Default::default()})]
    #[case(RenderOperation::RenderPreformattedLine(
        PreformattedLine{
            text: "asd".into(),
            alignment: Default::default(),
            block_length: 42,
            unformatted_length: 1337
        }
    ))]
    #[case(RenderOperation::RenderDynamic(Rc::new(Dynamic)))]
    fn same_not_modified(#[case] operation: RenderOperation) {
        let diff = operation.is_content_different(&operation);
        assert!(!diff);
    }

    #[test]
    fn different_text() {
        let lhs = RenderOperation::RenderTextLine { line: String::from("foo").into(), alignment: Default::default() };
        let rhs = RenderOperation::RenderTextLine { line: String::from("bar").into(), alignment: Default::default() };
        assert!(lhs.is_content_different(&rhs));
    }

    #[test]
    fn different_text_alignment() {
        let lhs = RenderOperation::RenderTextLine {
            line: String::from("foo").into(),
            alignment: Alignment::Left { margin: Margin::Fixed(42) },
        };
        let rhs = RenderOperation::RenderTextLine {
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
    fn no_slide_changes() {
        let presentation = Presentation::new(vec![
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
        ]);
        assert_eq!(PresentationDiffer::first_modified_slide(&presentation, &presentation), None);
    }

    #[test]
    fn slides_truncated() {
        let lhs = Presentation::new(vec![
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
        ]);
        let rhs = Presentation::new(vec![Slide { render_operations: vec![RenderOperation::ClearScreen] }]);

        assert_eq!(PresentationDiffer::first_modified_slide(&lhs, &rhs), Some(0));
    }

    #[test]
    fn slides_added() {
        let lhs = Presentation::new(vec![Slide { render_operations: vec![RenderOperation::ClearScreen] }]);
        let rhs = Presentation::new(vec![
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
        ]);

        assert_eq!(PresentationDiffer::first_modified_slide(&lhs, &rhs), Some(1));
    }

    #[test]
    fn second_slide_content_changed() {
        let lhs = Presentation::new(vec![
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
        ]);
        let rhs = Presentation::new(vec![
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
            Slide { render_operations: vec![RenderOperation::JumpToVerticalCenter] },
            Slide { render_operations: vec![RenderOperation::ClearScreen] },
        ]);

        assert_eq!(PresentationDiffer::first_modified_slide(&lhs, &rhs), Some(1));
    }

    #[test]
    fn presentation_changed_style() {
        let lhs = Presentation::new(vec![Slide {
            render_operations: vec![RenderOperation::SetColors(Colors {
                background: None,
                foreground: Some(Color::new(255, 0, 0)),
            })],
        }]);
        let rhs = Presentation::new(vec![Slide {
            render_operations: vec![RenderOperation::SetColors(Colors {
                background: None,
                foreground: Some(Color::new(0, 0, 0)),
            })],
        }]);

        assert_eq!(PresentationDiffer::first_modified_slide(&lhs, &rhs), None);
    }
}
