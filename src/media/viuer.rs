use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use image::{DynamicImage, GenericImageView};
use std::{fs, ops::Deref};

pub(crate) struct ViuerResource(DynamicImage);

impl ResourceProperties for ViuerResource {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }
}

impl From<DynamicImage> for ViuerResource {
    fn from(image: DynamicImage) -> Self {
        let image = image.into_rgba8();
        Self(image.into())
    }
}

impl Deref for ViuerResource {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sixel")]
#[derive(Default)]
pub(crate) enum SixelSupport {
    Enabled,
    #[default]
    Disabled,
}

#[derive(Default)]
pub struct ViuerPrinter {
    #[cfg(feature = "sixel")]
    sixel: SixelSupport,
}

impl ViuerPrinter {
    #[cfg(feature = "sixel")]
    pub(crate) fn new(sixel: SixelSupport) -> Self {
        Self { sixel }
    }
}

impl PrintImage for ViuerPrinter {
    type Resource = ViuerResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        Ok(ViuerResource(image))
    }

    fn register_resource<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(ViuerResource(image))
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
            x: options.cursor_position.column,
            y: options.cursor_position.row as i16,
            #[cfg(feature = "sixel")]
            use_sixel: matches!(self.sixel, SixelSupport::Enabled),
            ..Default::default()
        };
        viuer::print(&image.0, &config)?;
        Ok(())
    }
}
