use super::{
    Image, ImageSource,
    protocols::{
        ascii::{AsciiImage, AsciiPrinter},
        iterm::{ItermImage, ItermPrinter},
        kitty::{KittyImage, KittyMode, KittyPrinter},
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
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) trait PrintImage {
    type Image: ImageProperties;

    /// Register an image.
    fn register(&self, image: DynamicImage) -> Result<Self::Image, RegisterImageError>;

    /// Load and register an image from the given path.
    fn register_from_path<P: AsRef<Path>>(&self, path: P) -> Result<Self::Image, RegisterImageError>;

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
    #[cfg(feature = "sixel")]
    Sixel(super::protocols::sixel::SixelImage),
}

impl ImageProperties for TerminalImage {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Kitty(image) => image.dimensions(),
            Self::Iterm(image) => image.dimensions(),
            Self::Ascii(image) => image.dimensions(),
            #[cfg(feature = "sixel")]
            Self::Sixel(image) => image.dimensions(),
        }
    }
}

pub enum ImagePrinter {
    Kitty(KittyPrinter),
    Iterm(ItermPrinter),
    Ascii(AsciiPrinter),
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

    #[cfg(feature = "sixel")]
    fn new_sixel() -> Result<Self, CreatePrinterError> {
        Ok(Self::Sixel(super::protocols::sixel::SixelPrinter::new()?))
    }
}

impl PrintImage for ImagePrinter {
    type Image = TerminalImage;

    fn register(&self, image: DynamicImage) -> Result<Self::Image, RegisterImageError> {
        let image = match self {
            Self::Kitty(printer) => TerminalImage::Kitty(printer.register(image)?),
            Self::Iterm(printer) => TerminalImage::Iterm(printer.register(image)?),
            Self::Ascii(printer) => TerminalImage::Ascii(printer.register(image)?),
            Self::Null => return Err(RegisterImageError::Unsupported),
            #[cfg(feature = "sixel")]
            Self::Sixel(printer) => TerminalImage::Sixel(printer.register(image)?),
        };
        Ok(image)
    }

    fn register_from_path<P: AsRef<Path>>(&self, path: P) -> Result<Self::Image, RegisterImageError> {
        let image = match self {
            Self::Kitty(printer) => TerminalImage::Kitty(printer.register_from_path(path)?),
            Self::Iterm(printer) => TerminalImage::Iterm(printer.register_from_path(path)?),
            Self::Ascii(printer) => TerminalImage::Ascii(printer.register_from_path(path)?),
            Self::Null => return Err(RegisterImageError::Unsupported),
            #[cfg(feature = "sixel")]
            Self::Sixel(printer) => TerminalImage::Sixel(printer.register_from_path(path)?),
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
            #[cfg(feature = "sixel")]
            (Self::Sixel(printer), TerminalImage::Sixel(image)) => printer.print(image, options, terminal),
            _ => Err(PrintImageError::Unsupported),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ImageRegistry(pub Arc<ImagePrinter>);

impl fmt::Debug for ImageRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = match self.0.as_ref() {
            ImagePrinter::Kitty(_) => "Kitty",
            ImagePrinter::Iterm(_) => "Iterm",
            ImagePrinter::Ascii(_) => "Ascii",
            ImagePrinter::Null => "Null",
            #[cfg(feature = "sixel")]
            ImagePrinter::Sixel(_) => "Sixel",
        };
        write!(f, "ImageRegistry<{inner}>")
    }
}

impl ImageRegistry {
    pub(crate) fn register_image(&self, image: DynamicImage) -> Result<Image, RegisterImageError> {
        let resource = self.0.register(image)?;
        let image = Image::new(resource, ImageSource::Generated);
        Ok(image)
    }

    pub(crate) fn register_resource(&self, path: PathBuf) -> Result<Image, RegisterImageError> {
        let resource = self.0.register_from_path(&path)?;
        let image = Image::new(resource, ImageSource::Filesystem(path));
        Ok(image)
    }
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
