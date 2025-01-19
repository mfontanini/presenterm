use crate::render::properties::{CursorPosition, WindowSize};

/// Scale an image to a specific size.
pub(crate) fn scale_image(
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

    fit_image_to_window(window_dimensions, image_width as u32, image_height as u32, position)
}

/// Shrink an image so it fits the dimensions of the layout it's being displayed in.
pub(crate) fn fit_image_to_window(
    dimensions: &WindowSize,
    image_width: u32,
    image_height: u32,
    position: &CursorPosition,
) -> TerminalRect {
    let aspect_ratio = image_height as f64 / image_width as f64;

    // Compute the image's width in columns by translating pixels -> columns.
    let column_in_pixels = dimensions.pixels_per_column();
    let column_margin = (dimensions.columns as f64 * 0.95) as u32;
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
        width_in_columns = (width_in_columns as f64 * shrink_ratio).ceil() as u32;
    }
    // Don't go too far wide.
    let width_in_columns = width_in_columns.min(column_margin);
    let height_in_rows = (width_in_columns as f64 * aspect_ratio / 2.0) as u16;

    let width_in_columns = width_in_columns.max(1);
    let height_in_rows = height_in_rows.max(1);

    // Draw it in the middle
    let start_column = dimensions.columns / 2 - (width_in_columns / 2) as u16;
    let start_column = start_column + position.column;
    TerminalRect { start_column, columns: width_in_columns as u16, rows: height_in_rows }
}

#[derive(Debug)]
pub(crate) struct TerminalRect {
    pub(crate) start_column: u16,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}
