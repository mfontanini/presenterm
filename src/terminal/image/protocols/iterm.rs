use crate::terminal::{
    image::printer::{ImageProperties, ImageSpec, PrintImage, PrintImageError, PrintOptions, RegisterImageError},
    printer::{TerminalCommand, TerminalIo},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{GenericImageView, ImageEncoder, RgbaImage, codecs::png::PngEncoder};
use std::fs;

pub(crate) struct ItermImage {
    dimensions: (u32, u32),
    raw_length: usize,
    base64_contents: String,
}

impl ItermImage {
    fn new(contents: Vec<u8>, dimensions: (u32, u32)) -> Self {
        let raw_length = contents.len();
        let base64_contents = STANDARD.encode(&contents);
        Self { dimensions, raw_length, base64_contents }
    }

    pub(crate) fn as_rgba8(&self) -> RgbaImage {
        let contents = STANDARD.decode(&self.base64_contents).expect("base64 must be valid");
        let image = image::load_from_memory(&contents).expect("image must have been originally valid");
        image.to_rgba8()
    }
}

impl ImageProperties for ItermImage {
    fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

#[derive(Default)]
pub struct ItermPrinter;

impl PrintImage for ItermPrinter {
    type Image = ItermImage;

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        match spec {
            ImageSpec::Generated(image) => {
                let dimensions = image.dimensions();
                let mut contents = Vec::new();
                let encoder = PngEncoder::new(&mut contents);
                encoder.write_image(image.as_bytes(), dimensions.0, dimensions.1, image.color().into())?;
                Ok(ItermImage::new(contents, dimensions))
            }
            ImageSpec::Filesystem(path) => {
                let contents = fs::read(path)?;
                let image = image::load_from_memory(&contents)?;
                Ok(ItermImage::new(contents, image.dimensions()))
            }
        }
    }

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
    {
        let size = image.raw_length;
        let columns = options.columns;
        let rows = options.rows;
        let contents = &image.base64_contents;
        let content = format!(
            "\x1b]1337;File=size={size};width={columns};height={rows};inline=1;preserveAspectRatio=1:{contents}\x07"
        );
        terminal.execute(&TerminalCommand::PrintText { content: &content, style: Default::default() })?;
        Ok(())
    }
}
