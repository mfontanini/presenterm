use crossterm::terminal::window_size;
use std::io;

#[derive(Debug, Clone)]
pub struct WindowSize {
    pub rows: u16,
    pub columns: u16,
    pub height: u16,
    pub width: u16,
}

impl WindowSize {
    pub fn current() -> io::Result<Self> {
        let size = window_size()?;
        Ok(size.into())
    }

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

    pub fn pixels_per_column(&self) -> f64 {
        self.width as f64 / self.columns as f64
    }

    pub fn pixels_per_row(&self) -> f64 {
        self.height as f64 / self.rows as f64
    }
}

impl From<crossterm::terminal::WindowSize> for WindowSize {
    fn from(size: crossterm::terminal::WindowSize) -> Self {
        Self { rows: size.rows, columns: size.columns, width: size.width, height: size.height }
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

        let dimensions = dimensions.shrink_rows(3);
        assert_eq!(dimensions.rows, 7);
        assert_eq!(dimensions.height, 70);
    }
}
