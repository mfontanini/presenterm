use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{io, time::Duration};

#[derive(Default)]
pub struct UserInput {
    state: InputState,
}

impl UserInput {
    pub fn poll_next_command(&mut self, timeout: Duration) -> io::Result<Option<UserCommand>> {
        if poll(timeout)? { self.next_command() } else { Ok(None) }
    }

    pub fn next_command(&mut self) -> io::Result<Option<UserCommand>> {
        let command = match read()? {
            Event::Key(event) => self.handle_key_event(&event),
            Event::Resize(..) => Some(UserCommand::Redraw),
            _ => None,
        };
        if command.is_some() {
            self.state.reset();
        }
        Ok(command)
    }

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<UserCommand> {
        match event.code {
            KeyCode::Char('h') | KeyCode::Char('k') | KeyCode::Left | KeyCode::PageUp | KeyCode::Up => {
                Some(UserCommand::JumpPreviousSlide)
            }
            KeyCode::Char('l')
            | KeyCode::Char('j')
            | KeyCode::Right
            | KeyCode::PageDown
            | KeyCode::Down
            | KeyCode::Char(' ') => Some(UserCommand::JumpNextSlide),
            KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => Some(UserCommand::Exit),
            KeyCode::Char('G') => self.handle_uppercase_g(),
            KeyCode::Char('g') => self.handle_lowercase_g(),
            KeyCode::Char(number) if number.is_ascii_digit() => {
                let number = number.to_digit(10).expect("not a digit");
                self.handle_number(number);
                None
            }
            _ => {
                self.state.reset();
                None
            }
        }
    }

    fn handle_lowercase_g(&mut self) -> Option<UserCommand> {
        match self.state {
            InputState::PendingG => Some(UserCommand::JumpFirstSlide),
            InputState::Empty => {
                self.state = InputState::PendingG;
                None
            }
            _ => {
                self.state.reset();
                None
            }
        }
    }

    fn handle_uppercase_g(&mut self) -> Option<UserCommand> {
        match self.state {
            InputState::Empty => Some(UserCommand::JumpLastSlide),
            InputState::PendingNumber(number) => Some(UserCommand::JumpSlide(number)),
            _ => {
                self.state.reset();
                None
            }
        }
    }

    fn handle_number(&mut self, number: u32) {
        let maybe_next = match self.state {
            InputState::PendingNumber(current) => current.checked_mul(10).and_then(|n| n.checked_add(number)),
            InputState::Empty => Some(number),
            InputState::OverflowedNumber => {
                return;
            }
            _ => {
                self.state.reset();
                return;
            }
        };
        // If we overflowed, jump to a terminal state that indicates so. This way 123123123G is not
        // an alias for G
        match maybe_next {
            Some(number) => self.state = InputState::PendingNumber(number),
            None => self.state = InputState::OverflowedNumber,
        };
    }
}

#[derive(Clone, Debug)]
pub enum UserCommand {
    Redraw,
    JumpNextSlide,
    JumpPreviousSlide,
    JumpFirstSlide,
    JumpLastSlide,
    JumpSlide(u32),
    Exit,
}

#[derive(Default)]
enum InputState {
    #[default]
    Empty,
    PendingG,
    PendingNumber(u32),
    OverflowedNumber,
}

impl InputState {
    fn reset(&mut self) {
        *self = InputState::Empty;
    }
}
