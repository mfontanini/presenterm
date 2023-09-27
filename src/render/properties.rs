use crossterm::terminal::window_size;
use std::io;

#[derive(Debug, Clone)]
pub struct WindowSize {
    pub rows: u16,
    pub columns: u16,
    pub width: u16,
    pub height: u16,
}

impl WindowSize {
    pub fn current() -> io::Result<Self> {
        let size = window_size()?;
        Ok(size.into())
    }
}

impl From<crossterm::terminal::WindowSize> for WindowSize {
    fn from(size: crossterm::terminal::WindowSize) -> Self {
        Self { rows: size.rows, columns: size.columns, width: size.width, height: size.height }
    }
}
