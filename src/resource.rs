use crate::{
    render::media::{Image, InvalidImage},
    theme::{LoadThemeError, PresentationTheme},
};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

/// Manages resources pulled from the filesystem such as images.
///
/// All resources are cached so once a specific resource is loaded, looking it up with the same
/// path will involve an in-memory lookup.
pub struct Resources {
    base_path: PathBuf,
    images: HashMap<PathBuf, Image>,
    themes: HashMap<PathBuf, PresentationTheme>,
}

impl Resources {
    /// Construct a new resource manager over the provided based path.
    ///
    /// Any relative paths will be assumed to be relative to the given base.
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self { base_path: base_path.into(), images: Default::default(), themes: Default::default() }
    }

    /// Get the image at the given path.
    pub(crate) fn image<P: AsRef<Path>>(&mut self, path: P) -> Result<Image, LoadImageError> {
        let path = self.base_path.join(path);
        if let Some(image) = self.images.get(&path) {
            return Ok(image.clone());
        }

        let contents = fs::read(&path).map_err(|e| LoadImageError::Io(path.clone(), e))?;
        let image = Image::new(&contents)?;
        self.images.insert(path, image.clone());
        Ok(image)
    }

    /// Get the theme at the given path.
    pub(crate) fn theme<P: AsRef<Path>>(&mut self, path: P) -> Result<PresentationTheme, LoadThemeError> {
        let path = self.base_path.join(path);
        if let Some(theme) = self.themes.get(&path) {
            return Ok(theme.clone());
        }

        let theme = PresentationTheme::from_path(&path)?;
        self.themes.insert(path, theme.clone());
        Ok(theme)
    }

    /// Clears all resources.
    pub(crate) fn clear(&mut self) {
        self.images.clear();
        self.themes.clear();
    }
}

/// An error loading an image.
#[derive(thiserror::Error, Debug)]
pub enum LoadImageError {
    #[error("io error reading {0}: {1}")]
    Io(PathBuf, io::Error),

    #[error("processing image: {0}")]
    InvalidImage(#[from] InvalidImage),
}
