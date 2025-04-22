use super::{AnimateTransition, LinesFrame, TransitionDirection};
use crate::{
    WindowSize,
    markdown::elements::Line,
    terminal::virt::{TerminalGrid, TerminalRowIterator},
};

pub(crate) struct SlideHorizontalAnimation {
    grid: TerminalGrid,
    dimensions: WindowSize,
    direction: TransitionDirection,
}

impl SlideHorizontalAnimation {
    pub(crate) fn new(
        left: TerminalGrid,
        right: TerminalGrid,
        dimensions: WindowSize,
        direction: TransitionDirection,
    ) -> Self {
        let mut rows = Vec::new();
        for (mut row, right) in left.rows.into_iter().zip(right.rows) {
            row.extend(right);
            rows.push(row);
        }
        let grid = TerminalGrid { rows, background_color: left.background_color, images: Default::default() };
        Self { grid, dimensions, direction }
    }
}

impl AnimateTransition for SlideHorizontalAnimation {
    type Frame = LinesFrame;

    fn build_frame(&self, frame: usize, _previous_frame: usize) -> Self::Frame {
        let total = self.total_frames();
        let frame = frame.min(total);
        let index = match &self.direction {
            TransitionDirection::Next => frame,
            TransitionDirection::Previous => total.saturating_sub(frame),
        };
        let mut lines = Vec::new();
        for row in &self.grid.rows {
            let row = &row[index..index + self.dimensions.columns as usize];
            let mut line = Vec::new();
            let max_width = self.dimensions.columns as usize;
            let mut width = 0;
            for mut text in TerminalRowIterator::new(row) {
                let text_width = text.width() * text.style.size as usize;
                if width + text_width > max_width {
                    let capped_width = max_width.saturating_sub(width) / text.style.size as usize;
                    if capped_width == 0 {
                        continue;
                    }
                    text.content = text.content.chars().take(capped_width).collect();
                }
                width += text_width;
                line.push(text);
            }
            lines.push(Line(line));
        }
        LinesFrame { lines, background_color: self.grid.background_color }
    }

    fn total_frames(&self) -> usize {
        self.grid.rows[0].len().saturating_sub(self.dimensions.columns as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn as_text(line: Line) -> String {
        line.0.into_iter().map(|l| l.content).collect()
    }

    #[rstest]
    #[case::next_frame0(0, TransitionDirection::Next, &["AB", "CD"])]
    #[case::next_frame1(1, TransitionDirection::Next, &["BE", "DG"])]
    #[case::next_frame2(2, TransitionDirection::Next, &["EF", "GH"])]
    #[case::next_way_past(100, TransitionDirection::Next, &["EF", "GH"])]
    #[case::previous_frame0(0, TransitionDirection::Previous, &["EF", "GH"])]
    #[case::previous_frame1(1, TransitionDirection::Previous, &["BE", "DG"])]
    #[case::previous_frame2(2, TransitionDirection::Previous, &["AB", "CD"])]
    #[case::previous_way_past(100, TransitionDirection::Previous, &["AB", "CD"])]
    fn build_frame(#[case] frame: usize, #[case] direction: TransitionDirection, #[case] expected: &[&str]) {
        use crate::transitions::utils::build_grid;

        let left = build_grid(&["AB", "CD"]);
        let right = build_grid(&["EF", "GH"]);
        let dimensions = WindowSize { rows: 2, columns: 2, height: 0, width: 0 };
        let transition = SlideHorizontalAnimation::new(left, right, dimensions, direction);
        let lines: Vec<_> = transition.build_frame(frame, 0).lines.into_iter().map(as_text).collect();
        assert_eq!(lines, expected);
    }
}
