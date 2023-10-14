use crossterm::{cursor::position, terminal::window_size};
use std::io;

/// The size of the terminal window.
///
/// This is the same as [crossterm::terminal::window_size] except with some added functionality,
/// like implementing `Clone`.
#[derive(Debug, Clone)]
pub struct WindowSize {
    pub rows: u16,
    pub columns: u16,
    pub height: u16,
    pub width: u16,
}

impl WindowSize {
    /// Get the current window size.
    pub fn current() -> io::Result<Self> {
        let size = window_size()?;
        Ok(size.into())
    }

    /// Shrink a window by the given number of rows.
    ///
    /// This preserves the relationship between rows and pixels.
    pub fn shrink_rows(&self, amount: u16) -> WindowSize {
        let pixels_per_row = self.pixels_per_row();
        let height_to_shrink = (pixels_per_row * amount as f64) as u16;
        WindowSize {
            rows: self.rows.saturating_sub(amount),
            columns: self.columns,
            height: self.height.saturating_sub(height_to_shrink),
            width: self.width,
        }
    }

    /// Shrink a window by the given number of columns.
    ///
    /// This preserves the relationship between columns and pixels.
    pub fn shrink_columns(&self, amount: u16) -> WindowSize {
        let pixels_per_column = self.pixels_per_column();
        let width_to_shrink = (pixels_per_column * amount as f64) as u16;
        WindowSize {
            rows: self.rows,
            columns: self.columns.saturating_sub(amount),
            height: self.height,
            width: self.width.saturating_sub(width_to_shrink),
        }
    }

    /// The number of pixels per column.
    pub fn pixels_per_column(&self) -> f64 {
        self.width as f64 / self.columns as f64
    }

    /// The number of pixels per row.
    pub fn pixels_per_row(&self) -> f64 {
        self.height as f64 / self.rows as f64
    }
}

impl From<crossterm::terminal::WindowSize> for WindowSize {
    fn from(size: crossterm::terminal::WindowSize) -> Self {
        Self { rows: size.rows, columns: size.columns, width: size.width, height: size.height }
    }
}

/// The cursor's position.
#[derive(Debug, Clone, Default)]
pub struct CursorPosition {
    pub column: u16,
    pub row: u16,
}

impl CursorPosition {
    /// Get the current cursor position.
    pub fn current() -> io::Result<Self> {
        let (column, row) = position()?;
        Ok(Self { column, row })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn shrink() {
        let dimensions = WindowSize { rows: 10, columns: 10, width: 200, height: 100 };
        assert_eq!(dimensions.pixels_per_column(), 20.0);
        assert_eq!(dimensions.pixels_per_row(), 10.0);

        let new_dimensions = dimensions.shrink_rows(3);
        assert_eq!(new_dimensions.rows, 7);
        assert_eq!(new_dimensions.height, 70);

        let new_dimensions = new_dimensions.shrink_columns(3);
        assert_eq!(new_dimensions.columns, 7);
        assert_eq!(new_dimensions.width, 140);
    }
}
