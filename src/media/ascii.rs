use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use image::{DynamicImage, GenericImageView};
use std::{fs, ops::Deref};

pub(crate) struct AsciiResource(DynamicImage);

impl ResourceProperties for AsciiResource {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }
}

impl From<DynamicImage> for AsciiResource {
    fn from(image: DynamicImage) -> Self {
        let image = image.into_rgba8();
        Self(image.into())
    }
}

impl Deref for AsciiResource {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct AsciiPrinter;

impl PrintImage for AsciiPrinter {
    type Resource = AsciiResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        Ok(AsciiResource(image))
    }

    fn register_resource<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(AsciiResource(image))
    }

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, _writer: &mut W) -> Result<(), PrintImageError>
    where
        W: std::io::Write,
    {
        let config = viuer::Config {
            width: Some(options.columns as u32),
            height: Some(options.rows as u32),
            use_kitty: false,
            use_iterm: false,
            #[cfg(feature = "sixel")]
            use_sixel: false,
            x: options.cursor_position.column,
            y: options.cursor_position.row as i16,
            ..Default::default()
        };
        viuer::print(&image.0, &config)?;
        Ok(())
    }
}
