use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io;

#[derive(Default)]
pub struct Input {
    state: InputState,
}

impl Input {
    pub fn next_command(&mut self) -> io::Result<Option<Command>> {
        let command = match read()? {
            Event::Key(event) => self.handle_key_event(&event),
            Event::Resize(..) => Some(Command::Redraw),
            _ => None,
        };
        if command.is_some() {
            self.state.reset();
        }
        Ok(command)
    }

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<Command> {
        match event.code {
            KeyCode::Char('h') | KeyCode::Left | KeyCode::PageUp | KeyCode::Up => Some(Command::JumpPreviousSlide),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::PageDown | KeyCode::Down => Some(Command::JumpNextSlide),
            KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => Some(Command::Exit),
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

    fn handle_lowercase_g(&mut self) -> Option<Command> {
        match self.state {
            InputState::PendingG => Some(Command::JumpFirstSlide),
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

    fn handle_uppercase_g(&mut self) -> Option<Command> {
        match self.state {
            InputState::Empty => Some(Command::JumpLastSlide),
            InputState::PendingNumber(number) => Some(Command::JumpSlide(number)),
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

pub enum Command {
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
