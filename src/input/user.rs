use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{io, mem, time::Duration};

/// A user input handler.
#[derive(Default)]
pub(crate) struct UserInput {
    state: InputState,
}

impl UserInput {
    /// Polls for the next input command coming from the keyboard.
    pub(crate) fn poll_next_command(&mut self, timeout: Duration) -> io::Result<Option<UserCommand>> {
        if poll(timeout)? { self.next_command() } else { Ok(None) }
    }

    /// Blocks waiting for the next command.
    pub(crate) fn next_command(&mut self) -> io::Result<Option<UserCommand>> {
        let current_state = mem::take(&mut self.state);
        let (command, next_state) = match read()? {
            Event::Key(event) => Self::apply_key_event(event, current_state),
            Event::Resize(..) => (Some(UserCommand::Redraw), current_state),
            _ => (None, current_state),
        };
        self.state = next_state;
        Ok(command)
    }

    fn apply_key_event(event: KeyEvent, state: InputState) -> (Option<UserCommand>, InputState) {
        match event.code {
            KeyCode::Char('h') | KeyCode::Char('k') | KeyCode::Left | KeyCode::PageUp | KeyCode::Up => {
                (Some(UserCommand::JumpPreviousSlide), InputState::Empty)
            }
            KeyCode::Char('l')
            | KeyCode::Char('j')
            | KeyCode::Right
            | KeyCode::PageDown
            | KeyCode::Down
            | KeyCode::Char(' ') => (Some(UserCommand::JumpNextSlide), InputState::Empty),
            KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => {
                (Some(UserCommand::Exit), InputState::Empty)
            }
            KeyCode::Char('e') if event.modifiers == KeyModifiers::CONTROL => {
                (Some(UserCommand::RenderWidgets), InputState::Empty)
            }
            KeyCode::Char('G') => Self::apply_uppercase_g(state),
            KeyCode::Char('g') => Self::apply_lowercase_g(state),
            KeyCode::Char(number) if number.is_ascii_digit() => {
                let number = number.to_digit(10).expect("not a digit");
                (None, Self::apply_number(number, state))
            }
            _ => (None, InputState::Empty),
        }
    }

    fn apply_lowercase_g(state: InputState) -> (Option<UserCommand>, InputState) {
        match state {
            InputState::PendingG => (Some(UserCommand::JumpFirstSlide), InputState::Empty),
            InputState::Empty => (None, InputState::PendingG),
            _ => (None, InputState::Empty),
        }
    }

    fn apply_uppercase_g(state: InputState) -> (Option<UserCommand>, InputState) {
        match state {
            InputState::Empty => (Some(UserCommand::JumpLastSlide), InputState::Empty),
            InputState::PendingNumber(number) => (Some(UserCommand::JumpSlide(number)), InputState::Empty),
            _ => (None, InputState::Empty),
        }
    }

    fn apply_number(number: u32, state: InputState) -> InputState {
        let maybe_next = match state {
            InputState::PendingNumber(current) => current.checked_mul(10).and_then(|n| n.checked_add(number)),
            InputState::Empty => Some(number),
            _ => {
                return InputState::Empty;
            }
        };
        // If we overflowed, jump to a terminal state that indicates so. This way 123123123G is not
        // an alias for G
        match maybe_next {
            Some(number) => InputState::PendingNumber(number),
            None => InputState::OverflowedNumber,
        }
    }
}

/// A command from the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UserCommand {
    /// Redraw the presentation.
    ///
    /// This can happen on terminal resize.
    Redraw,

    /// Jump to the next slide.
    JumpNextSlide,

    /// Jump to the previous slide.
    JumpPreviousSlide,

    /// Jump to the first slide.
    JumpFirstSlide,

    /// Jump to the last slide.
    JumpLastSlide,

    /// Jump to one particular slide.
    JumpSlide(u32),

    /// Render any widgets in the currently visible slide.
    RenderWidgets,

    /// Exit the presentation.
    Exit,
}

#[derive(Default, Debug, PartialEq, Eq)]
enum InputState {
    #[default]
    Empty,
    PendingG,
    PendingNumber(u32),
    OverflowedNumber,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lowercase_g() {
        let state = InputState::Empty;
        let (command, state) = UserInput::apply_key_event(KeyCode::Char('g').into(), state);
        assert!(command.is_none());

        let (command, state) = UserInput::apply_key_event(KeyCode::Char('g').into(), state);
        assert_eq!(command, Some(UserCommand::JumpFirstSlide));
        assert_eq!(state, InputState::Empty);
    }

    #[test]
    fn uppercase_g() {
        let state = InputState::Empty;
        let (command, state) = UserInput::apply_key_event(KeyCode::Char('G').into(), state);
        assert_eq!(command, Some(UserCommand::JumpLastSlide));
        assert_eq!(state, InputState::Empty);
    }

    #[test]
    fn jump_number() {
        let state = InputState::Empty;
        let (command, state) = UserInput::apply_key_event(KeyCode::Char('1').into(), state);
        assert!(command.is_none());
        assert_eq!(state, InputState::PendingNumber(1));

        let (command, state) = UserInput::apply_key_event(KeyCode::Char('2').into(), state);
        assert!(command.is_none());
        assert_eq!(state, InputState::PendingNumber(12));

        let (command, state) = UserInput::apply_key_event(KeyCode::Char('G').into(), state);
        assert_eq!(command, Some(UserCommand::JumpSlide(12)));
        assert_eq!(state, InputState::Empty);
    }
}
