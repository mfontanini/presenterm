use crate::terminal::{
    image::printer::{ImageProperties, ImageSpec, PrintImage, PrintImageError, PrintOptions, RegisterImageError},
    printer::TerminalIo,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{GenericImageView, ImageEncoder, ImageFormat, codecs::png::PngEncoder};
use std::fs;

pub(crate) struct RawImage {
    contents: Vec<u8>,
    format: ImageFormat,
    width: u32,
    height: u32,
}

impl RawImage {
    pub(crate) fn to_inline_html(&self) -> String {
        let mime_type = self.format.to_mime_type();
        let data = STANDARD.encode(&self.contents);
        format!("data:{mime_type};base64,{data}")
    }
}

impl ImageProperties for RawImage {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

pub(crate) struct RawPrinter;

impl PrintImage for RawPrinter {
    type Image = RawImage;

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        let image = match spec {
            ImageSpec::Generated(image) => {
                let mut contents = Vec::new();
                let encoder = PngEncoder::new(&mut contents);
                let (width, height) = image.dimensions();
                encoder.write_image(image.as_bytes(), width, height, image.color().into())?;
                RawImage { contents, format: ImageFormat::Png, width, height }
            }
            ImageSpec::Filesystem(path) => {
                let contents = fs::read(path)?;
                let format = image::guess_format(&contents)?;
                let image = image::load_from_memory_with_format(&contents, format)?;
                let (width, height) = image.dimensions();
                RawImage { contents, format, width, height }
            }
        };
        Ok(image)
    }

    fn print<T>(&self, _image: &Self::Image, _options: &PrintOptions, _terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
    {
        Err(PrintImageError::Other("raw images can't be printed".into()))
    }
}
