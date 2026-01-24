use self::printer::{ImageProperties, TerminalImage};
use image::DynamicImage;
use protocols::ascii::AsciiImage;
use std::{
    fmt::Debug,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) mod printer;
pub(crate) mod protocols;
pub(crate) mod scale;

struct Inner {
    image: TerminalImage,
    ascii_image: Mutex<Option<AsciiImage>>,
}

/// An image.
///
/// This stores the image in an [std::sync::Arc] so it's cheap to clone.
#[derive(Clone)]
pub(crate) struct Image {
    inner: Arc<Inner>,
    pub(crate) source: ImageSource,
}

impl Image {
    /// Constructs a new image.
    pub(crate) fn new(image: TerminalImage, source: ImageSource) -> Self {
        let inner = Inner { image, ascii_image: Default::default() };
        Self { inner: Arc::new(inner), source }
    }

    pub(crate) fn to_ascii(&self) -> AsciiImage {
        let mut ascii_image = self.inner.ascii_image.lock().unwrap();
        match ascii_image.deref() {
            Some(image) => image.clone(),
            None => {
                let image = match &self.inner.image {
                    TerminalImage::Ascii(image) => image.clone(),
                    TerminalImage::Kitty(image) => DynamicImage::from(image.as_rgba8()).into(),
                    TerminalImage::Iterm(image) => DynamicImage::from(image.as_rgba8()).into(),
                    TerminalImage::Raw(_) => unreachable!("raw is only used for exports"),
                    TerminalImage::Sixel(image) => DynamicImage::from(image.as_rgba8()).into(),
                };
                *ascii_image = Some(image.clone());
                image
            }
        }
    }

    pub(crate) fn image(&self) -> &TerminalImage {
        &self.inner.image
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = self.inner.image.dimensions();
        write!(f, "Image<{width}x{height}>")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImageSource {
    Filesystem(PathBuf),
    Generated,
}
