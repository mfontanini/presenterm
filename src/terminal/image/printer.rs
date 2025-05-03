use super::{
    Image, ImageSource,
    protocols::{
        ascii::{AsciiImage, AsciiPrinter},
        iterm::{ItermImage, ItermPrinter},
        kitty::{KittyImage, KittyMode, KittyPrinter},
        raw::{RawImage, RawPrinter},
    },
};
use crate::{
    markdown::text_style::{Color, PaletteColorError},
    terminal::{
        GraphicsMode,
        printer::{TerminalError, TerminalIo},
    },
};
use image::{DynamicImage, ImageError};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt, io,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) trait PrintImage {
    type Image: ImageProperties;

    /// Register an image.
    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError>;

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo;
}

pub(crate) trait ImageProperties {
    fn dimensions(&self) -> (u32, u32);
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PrintOptions {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) z_index: i32,
    pub(crate) background_color: Option<Color>,
    // Width/height in pixels.
    #[allow(dead_code)]
    pub(crate) column_width: u16,
    #[allow(dead_code)]
    pub(crate) row_height: u16,
}

pub(crate) enum TerminalImage {
    Kitty(KittyImage),
    Iterm(ItermImage),
    Ascii(AsciiImage),
    Raw(RawImage),
    #[cfg(feature = "sixel")]
    Sixel(super::protocols::sixel::SixelImage),
}

impl ImageProperties for TerminalImage {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Kitty(image) => image.dimensions(),
            Self::Iterm(image) => image.dimensions(),
            Self::Ascii(image) => image.dimensions(),
            Self::Raw(image) => image.dimensions(),
            #[cfg(feature = "sixel")]
            Self::Sixel(image) => image.dimensions(),
        }
    }
}

pub enum ImagePrinter {
    Kitty(KittyPrinter),
    Iterm(ItermPrinter),
    Ascii(AsciiPrinter),
    Raw(RawPrinter),
    Null,
    #[cfg(feature = "sixel")]
    Sixel(super::protocols::sixel::SixelPrinter),
}

impl Default for ImagePrinter {
    fn default() -> Self {
        Self::new_ascii()
    }
}

impl ImagePrinter {
    pub fn new(mode: GraphicsMode) -> Result<Self, CreatePrinterError> {
        let printer = match mode {
            GraphicsMode::Kitty { mode, inside_tmux } => Self::new_kitty(mode, inside_tmux)?,
            GraphicsMode::Iterm2 => Self::new_iterm(),
            GraphicsMode::AsciiBlocks => Self::new_ascii(),
            GraphicsMode::Raw => Self::new_raw(),
            #[cfg(feature = "sixel")]
            GraphicsMode::Sixel => Self::new_sixel()?,
        };
        Ok(printer)
    }

    fn new_kitty(mode: KittyMode, inside_tmux: bool) -> io::Result<Self> {
        Ok(Self::Kitty(KittyPrinter::new(mode, inside_tmux)?))
    }

    fn new_iterm() -> Self {
        Self::Iterm(ItermPrinter)
    }

    fn new_ascii() -> Self {
        Self::Ascii(AsciiPrinter)
    }

    fn new_raw() -> Self {
        Self::Raw(RawPrinter)
    }

    #[cfg(feature = "sixel")]
    fn new_sixel() -> Result<Self, CreatePrinterError> {
        Ok(Self::Sixel(super::protocols::sixel::SixelPrinter::new()?))
    }
}

impl PrintImage for ImagePrinter {
    type Image = TerminalImage;

    fn register(&self, spec: ImageSpec) -> Result<Self::Image, RegisterImageError> {
        let image = match self {
            Self::Kitty(printer) => TerminalImage::Kitty(printer.register(spec)?),
            Self::Iterm(printer) => TerminalImage::Iterm(printer.register(spec)?),
            Self::Ascii(printer) => TerminalImage::Ascii(printer.register(spec)?),
            Self::Null => return Err(RegisterImageError::Unsupported),
            Self::Raw(printer) => TerminalImage::Raw(printer.register(spec)?),
            #[cfg(feature = "sixel")]
            Self::Sixel(printer) => TerminalImage::Sixel(printer.register(spec)?),
        };
        Ok(image)
    }

    fn print<T>(&self, image: &Self::Image, options: &PrintOptions, terminal: &mut T) -> Result<(), PrintImageError>
    where
        T: TerminalIo,
    {
        match (self, image) {
            (Self::Kitty(printer), TerminalImage::Kitty(image)) => printer.print(image, options, terminal),
            (Self::Iterm(printer), TerminalImage::Iterm(image)) => printer.print(image, options, terminal),
            (Self::Ascii(printer), TerminalImage::Ascii(image)) => printer.print(image, options, terminal),
            (Self::Null, _) => Ok(()),
            (Self::Raw(printer), TerminalImage::Raw(image)) => printer.print(image, options, terminal),
            #[cfg(feature = "sixel")]
            (Self::Sixel(printer), TerminalImage::Sixel(image)) => printer.print(image, options, terminal),
            _ => Err(PrintImageError::Unsupported),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ImageRegistry {
    printer: Arc<ImagePrinter>,
    images: Arc<Mutex<HashMap<PathBuf, Image>>>,
}

impl ImageRegistry {
    pub fn new(printer: Arc<ImagePrinter>) -> Self {
        Self { printer, images: Default::default() }
    }
}

impl fmt::Debug for ImageRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = match self.printer.as_ref() {
            ImagePrinter::Kitty(_) => "Kitty",
            ImagePrinter::Iterm(_) => "Iterm",
            ImagePrinter::Ascii(_) => "Ascii",
            ImagePrinter::Null => "Null",
            ImagePrinter::Raw(_) => "Raw",
            #[cfg(feature = "sixel")]
            ImagePrinter::Sixel(_) => "Sixel",
        };
        write!(f, "ImageRegistry<{inner}>")
    }
}

impl ImageRegistry {
    pub(crate) fn register(&self, spec: ImageSpec) -> Result<Image, RegisterImageError> {
        let mut images = self.images.lock().unwrap();
        let (source, cache_key) = match &spec {
            ImageSpec::Generated(_) => (ImageSource::Generated, None),
            ImageSpec::Filesystem(path) => {
                // Return if already cached
                if let Some(image) = images.get(path) {
                    return Ok(image.clone());
                }
                (ImageSource::Filesystem(path.clone()), Some(path.clone()))
            }
        };
        let resource = self.printer.register(spec)?;
        let image = Image::new(resource, source);
        if let Some(key) = cache_key {
            images.insert(key.clone(), image.clone());
        }
        Ok(image)
    }

    pub(crate) fn clear(&self) {
        self.images.lock().unwrap().clear();
    }
}

pub(crate) enum ImageSpec {
    Generated(DynamicImage),
    Filesystem(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CreatePrinterError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PrintImageError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("unsupported image type")]
    Unsupported,

    #[error("image decoding: {0}")]
    Image(#[from] ImageError),

    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl From<PaletteColorError> for PrintImageError {
    fn from(e: PaletteColorError) -> Self {
        Self::Other(e.to_string().into())
    }
}

impl From<TerminalError> for PrintImageError {
    fn from(e: TerminalError) -> Self {
        match e {
            TerminalError::Io(e) => Self::Io(e),
            TerminalError::Image(e) => e,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RegisterImageError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("image decoding: {0}")]
    Image(#[from] ImageError),

    #[error("printer can't register images")]
    Unsupported,
}

impl PrintImageError {
    pub(crate) fn other<S>(message: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self::Other(message.into())
    }
}
