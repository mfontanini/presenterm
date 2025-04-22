use super::{AnimateTransition, LinesFrame, TransitionDirection};
use crate::terminal::virt::TerminalGrid;

pub(crate) struct CollapseHorizontalAnimation {
    from: TerminalGrid,
    to: TerminalGrid,
}

impl CollapseHorizontalAnimation {
    pub(crate) fn new(left: TerminalGrid, right: TerminalGrid, direction: TransitionDirection) -> Self {
        let (from, to) = match direction {
            TransitionDirection::Next => (left, right),
            TransitionDirection::Previous => (right, left),
        };
        Self { from, to }
    }
}

impl AnimateTransition for CollapseHorizontalAnimation {
    type Frame = LinesFrame;

    fn build_frame(&self, frame: usize, _previous_frame: usize) -> Self::Frame {
        let mut rows = Vec::new();
        for (from, to) in self.from.rows.iter().zip(&self.to.rows) {
            // Take the first and last `frame` cells
            let to_prefix = to.iter().take(frame);
            let to_suffix = to.iter().rev().take(frame).rev();

            let total_rows_from = from.len() - frame * 2;
            let from = from.iter().skip(frame).take(total_rows_from);
            let row = to_prefix.chain(from).chain(to_suffix).copied().collect();
            rows.push(row)
        }
        let grid = TerminalGrid { rows, background_color: self.from.background_color, images: Default::default() };
        LinesFrame::from(&grid)
    }

    fn total_frames(&self) -> usize {
        self.from.rows[0].len() / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{markdown::elements::Line, transitions::utils::build_grid};
    use rstest::rstest;

    fn as_text(line: Line) -> String {
        line.0.into_iter().map(|l| l.content).collect()
    }

    #[rstest]
    #[case(0, &["ABCDEF"])]
    #[case(1, &["1BCDE6"])]
    #[case(2, &["12CD56"])]
    #[case(3, &["123456"])]
    fn transition(#[case] frame: usize, #[case] expected: &[&str]) {
        let left = build_grid(&["ABCDEF"]);
        let right = build_grid(&["123456"]);
        let transition = CollapseHorizontalAnimation::new(left, right, TransitionDirection::Next);
        let lines: Vec<_> = transition.build_frame(frame, 0).lines.into_iter().map(as_text).collect();
        assert_eq!(lines, expected);
    }
}
