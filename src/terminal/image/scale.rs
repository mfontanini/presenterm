use crate::render::properties::{CursorPosition, WindowSize};

pub(crate) trait ScaleImage {
    /// Scale an image to a specific size.
    fn scale_image(
        &self,
        scale_size: &WindowSize,
        window_dimensions: &WindowSize,
        image_width: u32,
        image_height: u32,
        position: &CursorPosition,
    ) -> TerminalRect;

    /// Shrink an image so it fits the dimensions of the layout it's being displayed in.
    fn fit_image_to_rect(
        &self,
        dimensions: &WindowSize,
        image_width: u32,
        image_height: u32,
        position: &CursorPosition,
    ) -> TerminalRect;
}

pub(crate) struct ImageScaler {
    horizontal_margin: f64,
}

impl ScaleImage for ImageScaler {
    fn scale_image(
        &self,
        scale_size: &WindowSize,
        window_dimensions: &WindowSize,
        image_width: u32,
        image_height: u32,
        position: &CursorPosition,
    ) -> TerminalRect {
        let aspect_ratio = image_height as f64 / image_width as f64;
        let column_in_pixels = scale_size.pixels_per_column();
        let width_in_columns = scale_size.columns;
        let image_width = width_in_columns as f64 * column_in_pixels;
        let image_height = image_width * aspect_ratio;

        self.fit_image_to_rect(window_dimensions, image_width as u32, image_height as u32, position)
    }

    fn fit_image_to_rect(
        &self,
        dimensions: &WindowSize,
        image_width: u32,
        image_height: u32,
        position: &CursorPosition,
    ) -> TerminalRect {
        let aspect_ratio = image_height as f64 / image_width as f64;

        // Compute the image's width in columns by translating pixels -> columns.
        let column_in_pixels = dimensions.pixels_per_column();
        let column_margin = (dimensions.columns as f64 * (1.0 - self.horizontal_margin)) as u32;
        let mut width_in_columns = (image_width as f64 / column_in_pixels) as u32;

        // Do the same for its height.
        let row_in_pixels = dimensions.pixels_per_row();
        let height_in_rows = (image_height as f64 / row_in_pixels) as u32;

        // If the image doesn't fit vertically, shrink it.
        let available_height = dimensions.rows.saturating_sub(position.row) as u32;
        if height_in_rows > available_height {
            // Because we only use the width to draw, here we scale the width based on how much we
            // need to shrink the height.
            let shrink_ratio = available_height as f64 / height_in_rows as f64;
            width_in_columns = (width_in_columns as f64 * shrink_ratio).round() as u32;
        }
        // Don't go too far wide.
        let width_in_columns = width_in_columns.min(column_margin);

        // Now translate width -> height by using the original aspect ratio + translate based on
        // the window size's aspect ratio.
        let height_in_rows = (width_in_columns as f64 * aspect_ratio * dimensions.aspect_ratio()).round() as u16;

        let width_in_columns = width_in_columns.max(1);
        let height_in_rows = height_in_rows.max(1);

        TerminalRect { columns: width_in_columns as u16, rows: height_in_rows }
    }
}

impl Default for ImageScaler {
    fn default() -> Self {
        Self { horizontal_margin: 0.05 }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct TerminalRect {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const WINDOW: WindowSize = WindowSize { rows: 50, columns: 100, height: 200, width: 200 };
    const SMALL_WINDOW: WindowSize = WindowSize { rows: 3, columns: 6, height: 10, width: 10 };
    const OTHER_RATIO: WindowSize = WindowSize { rows: 10, columns: 10, height: 10, width: 10 };

    #[rstest]
    #[case::squares(WINDOW, 100, 100, TerminalRect { columns: 50, rows: 25 })]
    #[case::squares_smaller(WINDOW, 50, 50, TerminalRect { columns: 25, rows: 13 })]
    #[case::square_too_large(WINDOW, 400, 400, TerminalRect { columns: 100, rows: 50 })]
    #[case::too_tall(WINDOW, 200, 400, TerminalRect { columns: 50, rows: 50 })]
    #[case::too_wide(WINDOW, 400, 200, TerminalRect { columns: 100, rows: 25 })]
    #[case::small(SMALL_WINDOW, 899, 872, TerminalRect { columns: 6, rows: 3 })]
    #[case::other_ratio(OTHER_RATIO, 100, 100, TerminalRect { columns: 10, rows: 10 })]
    fn image_fitting(
        #[case] window: WindowSize,
        #[case] width: u32,
        #[case] height: u32,
        #[case] expected: TerminalRect,
    ) {
        let cursor = CursorPosition::default();
        let rect = ImageScaler { horizontal_margin: 0.0 }.fit_image_to_rect(&window, width, height, &cursor);
        assert_eq!(rect, expected);
    }
}
