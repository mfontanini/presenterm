use super::printer::{PrintImage, PrintImageError, PrintOptions, RegisterImageError, ResourceProperties};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{GenericImageView, ImageEncoder, codecs::png::PngEncoder};
use std::{env, fs, path::Path};

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

pub struct ItermPrinter {
    // Whether this is iterm2. Otherwise it can be a terminal that _supports_ the iterm2 protocol.
    is_iterm: bool,
}

impl Default for ItermPrinter {
    fn default() -> Self {
        for key in ["TERM_PROGRAM", "LC_TERMINAL"] {
            if let Ok(value) = env::var(key) {
                if value.contains("iTerm") {
                    return Self { is_iterm: true };
                }
            }
        }
        Self { is_iterm: false }
    }
}

impl PrintImage for ItermPrinter {
    type Resource = ItermResource;

    fn register_image(&self, image: image::DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        let dimensions = image.dimensions();
        let mut contents = Vec::new();
        let encoder = PngEncoder::new(&mut contents);
        encoder.write_image(image.as_bytes(), dimensions.0, dimensions.1, image.color().into())?;
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
            "\x1b]1337;File=size={size};width={columns};height={rows};inline=1;preserveAspectRatio=0:{contents}\x07"
        )?;
        // iterm2 really respects what we say and leaves no space, whereas wezterm does leave an
        // extra line here.
        if self.is_iterm {
            writeln!(writer)?;
        }
        Ok(())
    }
}
