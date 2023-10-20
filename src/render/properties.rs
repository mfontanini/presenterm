use crossterm::{cursor::position, terminal};
use std::io::{self, ErrorKind};

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
    pub has_pixels: bool,
}

impl WindowSize {
    /// Get the current window size.
    pub fn current() -> io::Result<Self> {
        match terminal::window_size() {
            Ok(size) => Ok(size.into()),
            Err(e) if e.kind() == ErrorKind::Unsupported => {
                // Fall back to a `WindowSize` that doesn't have pixel support.
                let size = terminal::size()?;
                Ok(size.into())
            }
            Err(e) => Err(e),
        }
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
            has_pixels: self.has_pixels,
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
            has_pixels: self.has_pixels,
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
        Self { rows: size.rows, columns: size.columns, width: size.width, height: size.height, has_pixels: true }
    }
}

impl From<(u16, u16)> for WindowSize {
    fn from((columns, rows): (u16, u16)) -> Self {
        Self { columns, rows, width: 0, height: 0, has_pixels: false }
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
        let dimensions = WindowSize { rows: 10, columns: 10, width: 200, height: 100, has_pixels: true };
        assert_eq!(dimensions.pixels_per_column(), 20.0);
        assert_eq!(dimensions.pixels_per_row(), 10.0);

        let new_dimensions = dimensions.shrink_rows(3);
        assert_eq!(new_dimensions.rows, 7);
        assert_eq!(new_dimensions.height, 70);

        let new_dimensions = new_dimensions.shrink_columns(3);
        assert_eq!(new_dimensions.columns, 7);
        assert_eq!(new_dimensions.width, 140);

        assert!(new_dimensions.has_pixels);
    }
}
