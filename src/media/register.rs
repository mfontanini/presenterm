use super::{
    image::{Image, ImageSource},
    printer::{PrintImage, RegisterImageError},
};
use crate::ImagePrinter;
use image::DynamicImage;
use std::{path::PathBuf, rc::Rc};

#[derive(Clone, Default)]
pub struct ImageRegistry(pub Rc<ImagePrinter>);

impl ImageRegistry {
    pub(crate) fn register_image(&self, image: DynamicImage) -> Result<Image, RegisterImageError> {
        let resource = self.0.register_image(image)?;
        let image = Image::new(resource, ImageSource::Generated);
        Ok(image)
    }

    pub(crate) fn register_resource(&self, path: PathBuf) -> Result<Image, RegisterImageError> {
        let resource = self.0.register_resource(&path)?;
        let image = Image::new(resource, ImageSource::Filesystem(path));
        Ok(image)
    }
}
