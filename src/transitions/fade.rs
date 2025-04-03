use super::{AnimateTransition, AnimationFrame, TransitionDirection};
use crate::{
    markdown::text_style::TextStyle,
    terminal::{
        printer::TerminalCommand,
        virt::{StyledChar, TerminalGrid},
    },
};
use std::str;

pub(crate) struct FadeAnimation {
    changes: Vec<Change>,
}

impl FadeAnimation {
    pub(crate) fn new(left: TerminalGrid, right: TerminalGrid, direction: TransitionDirection) -> Self {
        let mut changes = Vec::new();
        let background = left.background_color;
        for (row, (left, right)) in left.rows.into_iter().zip(right.rows).enumerate() {
            for (column, (left, right)) in left.into_iter().zip(right).enumerate() {
                let character = match &direction {
                    TransitionDirection::Next => right,
                    TransitionDirection::Previous => left,
                };
                if left != right {
                    let StyledChar { character, mut style } = character;
                    // If we don't have an explicit background color fall back to the default
                    style.colors.background = style.colors.background.or(background);

                    let mut char_buffer = [0; 4];
                    let char_buffer_len = character.encode_utf8(&mut char_buffer).len() as u8;
                    changes.push(Change {
                        row: row as u16,
                        column: column as u16,
                        char_buffer,
                        char_buffer_len,
                        style,
                    });
                }
            }
        }
        fastrand::shuffle(&mut changes);
        Self { changes }
    }
}

impl AnimateTransition for FadeAnimation {
    type Frame = FadeCellsFrame;

    fn build_frame(&self, frame: usize, previous_frame: usize) -> Self::Frame {
        let last_frame = self.changes.len().saturating_sub(1);
        let previous_frame = previous_frame.min(last_frame);
        let frame_index = frame.min(self.changes.len());
        let changes = self.changes[previous_frame..frame_index].to_vec();
        FadeCellsFrame { changes }
    }

    fn total_frames(&self) -> usize {
        self.changes.len()
    }
}

#[derive(Debug)]
pub(crate) struct FadeCellsFrame {
    changes: Vec<Change>,
}

impl AnimationFrame for FadeCellsFrame {
    fn build_commands(&self) -> Vec<TerminalCommand> {
        let mut commands = Vec::new();
        for change in &self.changes {
            let Change { row, column, char_buffer, char_buffer_len, style } = change;
            let char_buffer_len = *char_buffer_len as usize;
            // SAFETY: this is an utf8 encoded char so it must be valid
            let content = str::from_utf8(&char_buffer[..char_buffer_len]).expect("invalid utf8");
            commands.push(TerminalCommand::MoveTo { row: *row, column: *column });
            commands.push(TerminalCommand::PrintText { content, style: *style });
        }
        commands
    }
}

#[derive(Clone, Debug)]
struct Change {
    row: u16,
    column: u16,
    char_buffer: [u8; 4],
    char_buffer_len: u8,
    style: TextStyle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WindowSize,
        terminal::{printer::TerminalIo, virt::VirtualTerminal},
    };
    use rstest::rstest;

    #[rstest]
    #[case::next(TransitionDirection::Next)]
    #[case::previous(TransitionDirection::Previous)]
    fn transition(#[case] direction: TransitionDirection) {
        let left = TerminalGrid {
            rows: vec![
                vec!['X'.into(), ' '.into(), 'B'.into()],
                vec!['C'.into(), StyledChar::new('X', TextStyle::default().size(2)), 'D'.into()],
            ],
            background_color: None,
            images: Default::default(),
        };
        let right = TerminalGrid {
            rows: vec![
                vec![' '.into(), 'A'.into(), StyledChar::new('B', TextStyle::default().bold())],
                vec![StyledChar::new('C', TextStyle::default().size(2)), ' '.into(), '🚀'.into()],
            ],
            background_color: None,
            images: Default::default(),
        };
        let expected = match direction {
            TransitionDirection::Next => right.clone(),
            TransitionDirection::Previous => left.clone(),
        };
        let dimensions = WindowSize { rows: 2, columns: 3, height: 0, width: 0 };
        let mut virt = VirtualTerminal::new(dimensions, Default::default());
        let animation = FadeAnimation::new(left, right, direction);
        for command in animation.build_frame(animation.total_frames(), 0).build_commands() {
            virt.execute(&command).expect("failed to run")
        }
        let output = virt.into_contents();
        assert_eq!(output, expected);
    }
}
