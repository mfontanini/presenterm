use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::png::PngEncoder, GenericImageView, ImageEncoder};
use std::{fs, path::Path};

pub(crate) struct ItermResource {
    dimensions: (u32, u32),
    raw_length: usize,
    base64_contents: String,
}

impl ItermResource {
    fn new(contents: Vec<u8>, dimensions: (u32, u32)) -> Self {
        let raw_length = contents.len();
        let base64_contents = STANDARD.encode(&contents);
        Self { dimensions, raw_length, base64_contents }
    }
}

impl ResourceProperties for ItermResource {
    fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

#[derive(Default)]
pub struct ItermPrinter;

impl PrintImage for ItermPrinter {
    type Resource = ItermResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        let dimensions = image.dimensions();
        let mut contents = Vec::new();
        let encoder = PngEncoder::new(&mut contents);
        encoder.write_image(image.as_bytes(), dimensions.0, dimensions.1, image.color())?;
        Ok(ItermResource::new(contents, dimensions))
    }

    fn register_resource<P: AsRef<Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        Ok(ItermResource::new(contents, image.dimensions()))
    }

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: std::io::Write,
    {
        let size = image.raw_length;
        let columns = options.columns;
        let rows = options.rows;
        let contents = &image.base64_contents;
        write!(
            writer,
            "\x1b]1337;File=size={size};width={columns};height={rows};inline=1;preserveAspectRatio=1:{contents}\x07"
        )?;
        Ok(())
    }
}
