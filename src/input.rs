use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io;

pub struct Input;

impl Input {
    pub fn next_command() -> io::Result<Option<Command>> {
        match read()? {
            Event::Key(event) => Ok(Self::handle_key_event(&event)),
            Event::Resize(..) => Ok(Some(Command::Redraw)),
            _ => Ok(None),
        }
    }

    fn handle_key_event(event: &KeyEvent) -> Option<Command> {
        match event.code {
            KeyCode::Char('h') | KeyCode::Left | KeyCode::PageUp | KeyCode::Up => Some(Command::PreviousSlide),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::PageDown | KeyCode::Down => Some(Command::NextSlide),
            KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => Some(Command::Exit),
            _ => None,
        }
    }
}

pub enum Command {
    Redraw,
    NextSlide,
    PreviousSlide,
    Exit,
}
