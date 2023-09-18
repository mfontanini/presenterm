use crossterm::{cursor, terminal::WindowSize};
use image::{DynamicImage, ImageError};
use std::{fmt::Debug, fs, io, rc::Rc};
use viuer::ViuError;

#[derive(Clone)]
pub struct Image(Rc<DynamicImage>);

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image<{}x{}>", self.0.width(), self.0.height())
    }
}

impl Image {
    pub fn new(contents: &[u8]) -> Result<Self, InvalidImage> {
        let contents = image::load_from_memory(contents)?;
        let contents = Rc::new(contents);
        Ok(Self(contents))
    }
}

pub struct MediaDrawer;

impl MediaDrawer {
    pub fn draw_image(&self, image: &Image, dimensions: &WindowSize) -> Result<(), DrawImageError> {
        let position = cursor::position()?;
        let image = &image.0;

        // Compute the image's width in columns by translating pixels -> columns.
        let column_in_pixels = dimensions.width as f64 / dimensions.columns as f64;
        let column_margin = (dimensions.columns as f64 * 0.95) as u32;
        let mut width_in_columns = (image.width() as f64 / column_in_pixels) as u32;

        // Do the same for its height.
        let row_in_pixels = dimensions.height as f64 / dimensions.rows as f64;
        let height_in_rows = (image.height() as f64 / row_in_pixels) as u32;

        // If the image doesn't fit vertically, shrink it.
        let available_height = dimensions.rows.saturating_sub(position.1) as u32;
        if height_in_rows > available_height {
            // Because we only use the width to draw, here we scale the width based on how much we
            // need to shrink the height.
            let shrink_ratio = available_height as f64 / height_in_rows as f64;
            width_in_columns = (width_in_columns as f64 * shrink_ratio) as u32;
        }
        // Don't go too far wide.
        let width_in_columns = width_in_columns.min(column_margin);

        // Draw it in the middle
        let start_column = dimensions.columns / 2 - (width_in_columns / 2) as u16;
        let config = viuer::Config {
            width: Some(width_in_columns),
            x: start_column,
            y: position.1 as i16,
            ..Default::default()
        };
        self.clear_viuer_temp_files();
        viuer::print(image, &config)?;
        Ok(())
    }

    // viuer leaves a bunch of tempfiles when using kitty, this clears them up. Note that because
    // kitty is optional and this is technically not needed for this app to work, we swallow all
    // errors here.
    //
    // See https://github.com/atanunq/viuer/issues/47
    fn clear_viuer_temp_files(&self) {
        let Ok(entries) = fs::read_dir("/tmp") else { return };
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|f| f.to_str()) else { continue };
            if file_name.starts_with(".tmp.viuer.") {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub struct InvalidImage(#[from] ImageError);

#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub enum DrawImageError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("draw: {0}")]
    Draw(#[from] ViuError),

    #[error("invalid image: {0}")]
    InvalidImage(#[from] InvalidImage),
}
