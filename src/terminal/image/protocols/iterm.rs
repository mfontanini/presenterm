use crate::terminal::{
    image::printer::{ImageProperties, ImageSpec, PrintImage, PrintImageError, PrintOptions, RegisterImageError},
    printer::{TerminalCommand, TerminalIo},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{GenericImageView, ImageEncoder, RgbaImage, codecs::png::PngEncoder};
use std::{fs, str};

const CHUNK_SIZE: usize = 32 * 1024;

pub(crate) struct ItermImage {
    dimensions: (u32, u32),
    raw_length: usize,
    base64_contents: String,
}

impl ItermImage {
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

pub enum ItermMode {
    Single,
    Multipart,
}

pub struct ItermPrinter {
    mode: ItermMode,
    tmux: bool,
}

impl ItermPrinter {
    pub(crate) fn new(mode: ItermMode, tmux: bool) -> Self {
        Self { mode, tmux }
    }
}

impl PrintImage for ItermPrinter {
    type Image = ItermImage;

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        let (contents, dimensions) = match spec {
            ImageSpec::Generated(image) => {
                let dimensions = image.dimensions();
                let mut contents = Vec::new();
                let encoder = PngEncoder::new(&mut contents);
                encoder.write_image(image.as_bytes(), dimensions.0, dimensions.1, image.color().into())?;
                (contents, dimensions)
            }
            ImageSpec::Filesystem(path) => {
                let contents = fs::read(path)?;
                let image = image::load_from_memory(&contents)?;
                (contents, image.dimensions())
            }
        };
        let raw_length = contents.len();
        let contents = STANDARD.encode(&contents);
        Ok(ItermImage { dimensions, raw_length, base64_contents: contents })
    }

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
    {
        let size = image.raw_length;
        let columns = options.columns;
        let rows = options.rows;
        let (start, end) = match self.tmux {
            true => ("\x1bPtmux;\x1b\x1b]1337;", "\x07\x1b\\"),
            false => ("\x1b]1337;", "\x07"),
        };
        let base64 = &image.base64_contents;
        match &self.mode {
            ItermMode::Single => {
                let content = &format!(
                    "{start}File=size={size};width={columns};height={rows};inline=1;preserveAspectRatio=1:{base64}{end}"
                );
                terminal.execute(&TerminalCommand::PrintText { content, style: Default::default() })?;
            }
            ItermMode::Multipart => {
                let content = &format!(
                    "{start}MultipartFile=size={size};width={columns};height={rows};inline=1;preserveAspectRatio=1{end}"
                );
                terminal.execute(&TerminalCommand::PrintText { content, style: Default::default() })?;
                for chunk in base64.as_bytes().chunks(CHUNK_SIZE) {
                    // SAFETY: this is base64 so it must be utf8
                    let chunk = str::from_utf8(chunk).expect("not utf8");
                    let content = &format!("{start}FilePart={chunk}{end}");
                    terminal.execute(&TerminalCommand::PrintText { content, style: Default::default() })?;
                }
                terminal.execute(&TerminalCommand::PrintText {
                    content: &format!("{start}FileEnd{end}"),
                    style: Default::default(),
                })?;
            }
        };
        Ok(())
    }
}
