use crate::render::media::{Image, InvalidImage};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

pub struct Resources {
    base_path: PathBuf,
    images: HashMap<PathBuf, Image>,
}

impl Resources {
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self { base_path: base_path.into(), images: Default::default() }
    }

    pub fn image<P: AsRef<Path>>(&mut self, path: P) -> Result<Image, LoadImageError> {
        let path = self.base_path.join(path);
        if let Some(image) = self.images.get(&path) {
            return Ok(image.clone());
        }

        let contents = fs::read(&path).map_err(|e| LoadImageError::Io(path.clone(), e))?;
        let image = Image::new(&contents)?;
        self.images.insert(path, image.clone());
        Ok(image)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoadImageError {
    #[error("io error opening {0}: {1}")]
    Io(PathBuf, io::Error),

    #[error("processing image: {0}")]
    InvalidImage(#[from] InvalidImage),
}
