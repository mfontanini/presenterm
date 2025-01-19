use self::printer::{ImageProperties, TerminalImage};
use std::{fmt::Debug, ops::Deref, path::PathBuf, sync::Arc};

pub(crate) mod printer;
pub(crate) mod protocols;
pub(crate) mod scale;

/// An image.
///
/// This stores the image in an [std::sync::Arc] so it's cheap to clone.
#[derive(Clone)]
pub(crate) struct Image {
    pub(crate) image: Arc<TerminalImage>,
    pub(crate) source: ImageSource,
}

impl Image {
    /// Constructs a new image.
    pub(crate) fn new(image: TerminalImage, source: ImageSource) -> Self {
        Self { image: Arc::new(image), source }
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = self.image.dimensions();
        write!(f, "Image<{width}x{height}>")
    }
}

impl Deref for Image {
    type Target = TerminalImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImageSource {
    Filesystem(PathBuf),
    Generated,
}
