use crate::media::printer::{ImageResource, ResourceProperties};
use std::{fmt::Debug, ops::Deref, path::PathBuf, rc::Rc};

/// An image.
///
/// This stores the image in an [std::rc::Rc] so it's cheap to clone.
#[derive(Clone)]
pub(crate) struct Image {
    pub(crate) resource: Rc<ImageResource>,
    pub(crate) source: ImageSource,
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = self.resource.dimensions();
        write!(f, "Image<{width}x{height}>")
    }
}

impl Image {
    /// Constructs a new image.
    pub(crate) fn new(resource: ImageResource, source: ImageSource) -> Self {
        Self { resource: Rc::new(resource), source }
    }
}

impl Deref for Image {
    type Target = ImageResource;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImageSource {
    Filesystem(PathBuf),
    Generated,
}
