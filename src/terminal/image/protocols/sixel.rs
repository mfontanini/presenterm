use crate::terminal::{
    image::printer::{
        CreatePrinterError, ImageProperties, ImageSpec, PrintImage, PrintImageError, PrintOptions, RegisterImageError,
    },
    printer::{TerminalCommand, TerminalIo},
};
use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use sixel_rs::{
    encoder::{Encoder, QuickFrameBuilder},
    optflags::EncodePolicy,
    sys::PixelFormat,
};
use std::fs;

pub(crate) struct SixelImage(DynamicImage);

impl SixelImage {
    pub(crate) fn as_rgba8(&self) -> RgbaImage {
        self.0.to_rgba8()
    }
}

impl ImageProperties for SixelImage {
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
    type Image = SixelImage;

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        match spec {
            ImageSpec::Generated(image) => Ok(SixelImage(image)),
            ImageSpec::Filesystem(path) => {
                let contents = fs::read(path)?;
                let image = image::load_from_memory(&contents)?;
                Ok(SixelImage(image))
            }
        }
    }

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
    {
        // We're already positioned in the right place but we may not have flushed that yet.
        terminal.execute(&TerminalCommand::Flush)?;

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
