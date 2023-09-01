use crate::media::Image;
use std::{collections::HashMap, fs, io};

#[derive(Default)]
pub struct Resources {
    images: HashMap<String, Image>,
}

impl Resources {
    pub fn image(&mut self, url: &str) -> io::Result<Image> {
        if let Some(image) = self.images.get(url) {
            return Ok(image.clone());
        }

        let contents = fs::read(url)?;
        let image = Image::new(contents);
        self.images.insert(url.to_string(), image.clone());
        Ok(image)
    }
}
