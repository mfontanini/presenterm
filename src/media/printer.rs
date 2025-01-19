use super::{
    ascii::{AsciiPrinter, AsciiResource},
    graphics::GraphicsMode,
    iterm::{ItermPrinter, ItermResource},
    kitty::{KittyMode, KittyPrinter, KittyResource},
};
use crate::{render::properties::CursorPosition, style::Color};
use image::{DynamicImage, ImageError};
use std::{borrow::Cow, io, path::Path};

pub(crate) trait PrintImage {
    type Resource: ResourceProperties;

    /// Register an image.
    fn register_image(&self, image: DynamicImage) -> Result<Self::Resource, RegisterImageError>;

    /// Load and register a resource from the given path.
    fn register_resource<P: AsRef<Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError>;

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: io::Write;
}

pub(crate) trait ResourceProperties {
    fn dimensions(&self) -> (u32, u32);
}

#[derive(Debug)]
pub(crate) struct PrintOptions {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) cursor_position: CursorPosition,
    pub(crate) z_index: i32,
    pub(crate) background_color: Option<Color>,
    // Width/height in pixels.
    #[allow(dead_code)]
    pub(crate) column_width: u16,
    #[allow(dead_code)]
    pub(crate) row_height: u16,
}

pub(crate) enum ImageResource {
    Kitty(KittyResource),
    Iterm(ItermResource),
    Ascii(AsciiResource),
    #[cfg(feature = "sixel")]
    Sixel(super::sixel::SixelResource),
}

impl ResourceProperties for ImageResource {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Kitty(resource) => resource.dimensions(),
            Self::Iterm(resource) => resource.dimensions(),
            Self::Ascii(resource) => resource.dimensions(),
            #[cfg(feature = "sixel")]
            Self::Sixel(resource) => resource.dimensions(),
        }
    }
}

pub enum ImagePrinter {
    Kitty(KittyPrinter),
    Iterm(ItermPrinter),
    Ascii(AsciiPrinter),
    Null,
    #[cfg(feature = "sixel")]
    Sixel(super::sixel::SixelPrinter),
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
        Self::Iterm(ItermPrinter::default())
    }

    fn new_ascii() -> Self {
        Self::Ascii(AsciiPrinter)
    }

    #[cfg(feature = "sixel")]
    fn new_sixel() -> Result<Self, CreatePrinterError> {
        Ok(Self::Sixel(super::sixel::SixelPrinter::new()?))
    }
}

impl PrintImage for ImagePrinter {
    type Resource = ImageResource;

    fn register_image(&self, image: DynamicImage) -> Result<Self::Resource, RegisterImageError> {
        let resource = match self {
            Self::Kitty(printer) => ImageResource::Kitty(printer.register_image(image)?),
            Self::Iterm(printer) => ImageResource::Iterm(printer.register_image(image)?),
            Self::Ascii(printer) => ImageResource::Ascii(printer.register_image(image)?),
            Self::Null => return Err(RegisterImageError::Unsupported),
            #[cfg(feature = "sixel")]
            Self::Sixel(printer) => ImageResource::Sixel(printer.register_image(image)?),
        };
        Ok(resource)
    }

    fn register_resource<P: AsRef<Path>>(&self, path: P) -> Result<Self::Resource, RegisterImageError> {
        let resource = match self {
            Self::Kitty(printer) => ImageResource::Kitty(printer.register_resource(path)?),
            Self::Iterm(printer) => ImageResource::Iterm(printer.register_resource(path)?),
            Self::Ascii(printer) => ImageResource::Ascii(printer.register_resource(path)?),
            Self::Null => return Err(RegisterImageError::Unsupported),
            #[cfg(feature = "sixel")]
            Self::Sixel(printer) => ImageResource::Sixel(printer.register_resource(path)?),
        };
        Ok(resource)
    }

    fn print<W>(&self, image: &Self::Resource, options: &PrintOptions, writer: &mut W) -> Result<(), PrintImageError>
    where
        W: io::Write,
    {
        match (self, image) {
            (Self::Kitty(printer), ImageResource::Kitty(image)) => printer.print(image, options, writer),
            (Self::Iterm(printer), ImageResource::Iterm(image)) => printer.print(image, options, writer),
            (Self::Ascii(printer), ImageResource::Ascii(image)) => printer.print(image, options, writer),
            (Self::Null, _) => Ok(()),
            #[cfg(feature = "sixel")]
            (Self::Sixel(printer), ImageResource::Sixel(image)) => printer.print(image, options, writer),
            _ => Err(PrintImageError::Unsupported),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreatePrinterError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum PrintImageError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("unsupported image type")]
    Unsupported,

    #[error("image decoding: {0}")]
    Image(#[from] ImageError),

    #[error("other: {0}")]
    Other(Cow<'static, str>),
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterImageError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("image decoding: {0}")]
    Image(#[from] ImageError),

    #[error("printer can't register resources")]
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
