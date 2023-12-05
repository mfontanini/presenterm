use crate::render::properties::WindowSize;
use image::{DynamicImage, ImageError};
use std::{fmt::Debug, io, ops::Deref, path::PathBuf, rc::Rc};
use viuer::{is_iterm_supported, ViuError};

use super::properties::CursorPosition;

/// An image.
///
/// This stores the image in an [std::rc::Rc] so it's cheap to clone.
#[derive(Clone, PartialEq)]
pub(crate) struct Image {
    contents: Rc<DynamicImage>,
    pub(crate) source: ImageSource,
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image<{}x{}>", self.contents.width(), self.contents.height())
    }
}

impl Image {
    /// Constructs a new image.
    pub(crate) fn new(image: DynamicImage) -> Self {
        Self { contents: Rc::new(image), source: ImageSource::Generated }
    }

    /// Decodes an image from a byte sequence.
    pub(crate) fn decode(contents: &[u8], source: ImageSource) -> Result<Self, InvalidImage> {
        let contents = image::load_from_memory(contents)?;
        let contents = Rc::new(contents);
        Ok(Self { contents, source })
    }
}

impl Deref for Image {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.contents
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImageSource {
    Filesystem(PathBuf),
    Generated,
}

/// A media render.
pub(crate) struct MediaRender {
    mode: TerminalMode,
}

impl MediaRender {
    /// Draw an image.
    ///
    /// This will use the current terminal size and try to render the image where the cursor is
    /// currently positioned, respecting the image size. That is, if the image is 300 by 100 pixels
    /// and that fits in the screen at the current cursor positioned, it will be drawn as-is.
    ///
    /// In case the image does not fit, it will be resized to fit the screen, preserving the aspect
    /// ratio.
    pub(crate) fn draw_image(
        &self,
        image: &Image,
        position: CursorPosition,
        dimensions: &WindowSize,
    ) -> Result<(), RenderImageError> {
        if !dimensions.has_pixels {
            return Err(RenderImageError::NoWindowSize);
        }
        let source = &image.source;
        let image = &image.contents;

        // Compute the image's width in columns by translating pixels -> columns.
        let column_in_pixels = dimensions.pixels_per_column();
        let column_margin = (dimensions.columns as f64 * 0.95) as u32;
        let mut width_in_columns = (image.width() as f64 / column_in_pixels) as u32;

        // Do the same for its height.
        let row_in_pixels = dimensions.pixels_per_row();
        let height_in_rows = (image.height() as f64 / row_in_pixels) as u32;

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

        // Draw it in the middle
        let start_column = dimensions.columns / 2 - (width_in_columns / 2) as u16;
        let start_column = start_column + position.column;
        let config = viuer::Config {
            width: Some(width_in_columns),
            x: start_column,
            y: position.row as i16,
            ..Default::default()
        };
        // If we're using the iterm2 protocol, print this from the file as that makes images print
        // faster _and_ it causes gifs to be animated.
        //
        // This switch is because otherwise `viuer::print_from_file` for kitty/ascii blocks will
        // re-read the image every time.
        match (&self.mode, source) {
            (TerminalMode::Iterm2, ImageSource::Filesystem(image_path)) => viuer::print_from_file(image_path, &config)?,
            (TerminalMode::Other, _) | (_, ImageSource::Generated) => viuer::print(image, &config)?,
        };
        Ok(())
    }
}

impl Default for MediaRender {
    fn default() -> Self {
        let mode = match is_iterm_supported() {
            true => TerminalMode::Iterm2,
            false => TerminalMode::Other,
        };
        Self { mode }
    }
}

enum TerminalMode {
    Iterm2,
    Other,
}

/// An invalid image.
#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub struct InvalidImage(#[from] ImageError);

/// An image render error.
#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub enum RenderImageError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("draw: {0}")]
    Draw(#[from] ViuError),

    #[error("invalid image: {0}")]
    InvalidImage(#[from] InvalidImage),

    #[error("no window size support in terminal")]
    NoWindowSize,
}
