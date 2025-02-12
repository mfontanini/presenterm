use crossterm::terminal;
use std::io::{self, ErrorKind};

/// The size of the terminal window.
///
/// This is the same as [crossterm::terminal::window_size] except with some added functionality,
/// like implementing `Clone`.
#[derive(Debug, Clone)]
pub(crate) struct WindowSize {
    pub(crate) rows: u16,
    pub(crate) columns: u16,
    pub(crate) height: u16,
    pub(crate) width: u16,
}

impl WindowSize {
    /// Get the current window size.
    pub(crate) fn current(font_size_fallback: u8) -> io::Result<Self> {
        let mut size: Self = match terminal::window_size() {
            Ok(size) => size.into(),
            Err(e) if e.kind() == ErrorKind::Unsupported => {
                // Fall back to a `WindowSize` that doesn't have pixel support.
                let size = terminal::size()?;
                size.into()
            }
            Err(e) => return Err(e),
        };
        let font_size_fallback = font_size_fallback as u16;
        if size.width == 0 {
            size.width = size.columns * font_size_fallback.max(1);
        }
        if size.height == 0 {
            size.height = size.rows * font_size_fallback.max(1) * 2;
        }
        Ok(size)
    }

    /// Shrink a window by the given number of rows.
    ///
    /// This preserves the relationship between rows and pixels.
    pub(crate) fn shrink_rows(&self, amount: u16) -> WindowSize {
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
    pub(crate) fn shrink_columns(&self, amount: u16) -> WindowSize {
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
    pub(crate) fn pixels_per_column(&self) -> f64 {
        self.width as f64 / self.columns as f64
    }

    /// The number of pixels per row.
    pub(crate) fn pixels_per_row(&self) -> f64 {
        self.height as f64 / self.rows as f64
    }

    /// The aspect ratio for this size.
    pub(crate) fn aspect_ratio(&self) -> f64 {
        (self.rows as f64 / self.height as f64) / (self.columns as f64 / self.width as f64)
    }
}

impl From<crossterm::terminal::WindowSize> for WindowSize {
    fn from(size: crossterm::terminal::WindowSize) -> Self {
        Self { rows: size.rows, columns: size.columns, width: size.width, height: size.height }
    }
}

impl From<(u16, u16)> for WindowSize {
    fn from((columns, rows): (u16, u16)) -> Self {
        Self { columns, rows, width: 0, height: 0 }
    }
}

/// The cursor's position.
#[derive(Debug, Clone, Default)]
pub(crate) struct CursorPosition {
    pub(crate) column: u16,
    pub(crate) row: u16,
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
