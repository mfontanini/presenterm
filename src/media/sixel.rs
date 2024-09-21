use super::printer::{
    CreatePrinterError, PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties,
};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use sixel_rs::{
    encoder::{Encoder, QuickFrameBuilder},
    optflags::EncodePolicy,
    sys::PixelFormat,
};
use std::{fs, io};

pub(crate) struct SixelResource(DynamicImage);

impl ResourceProperties for SixelResource {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }
}

#[derive(Default)]
pub struct SixelPrinter;

impl SixelPrinter {
    pub(crate) fn new() -> Result<Self, CreatePrinterError> {
        Ok(Self)
    }
}

impl PrintImage for SixelPrinter {
    type Resource = SixelResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        Ok(SixelResource(image))
    }

    fn register_resource<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(SixelResource(image))
    }

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        // We're already positioned in the right place but we may not have flushed that yet.
        writer.flush()?;

        let encoder = Encoder::new().map_err(|e| PrintImageError::other(format!("creating sixel encoder: {e:?}")))?;
        encoder
            .set_encode_policy(EncodePolicy::Fast)
            .map_err(|e| PrintImageError::other(format!("setting encoder policy: {e:?}")))?;

        // This check was taken from viuer: it seems to be a bug in xterm
        let width = (options.column_width * options.columns).min(1000);
        let height = options.row_height * options.rows;
        let image = image.0.resize_exact(width as u32, height as u32, FilterType::Triangle);
        let bytes = image.into_rgba8().into_raw();

        let frame = QuickFrameBuilder::new()
            .width(width as usize)
            .height(height as usize)
            .format(PixelFormat::RGBA8888)
            .pixels(bytes);

        encoder.encode_bytes(frame).map_err(|e| PrintImageError::other(format!("encoding sixel image: {e:?}")))?;
        Ok(())
    }
}
