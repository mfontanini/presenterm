use crate::{
    markdown::elements::{Percent, PercentParseError, SourcePosition},
    presentation::builder::{
        BuildResult, PresentationBuilder,
        error::{BuildError, InvalidPresentation},
    },
    render::{
        operation::{AsRenderOperations, ImagePosition, ImageRenderProperties, ImageSize, RenderOperation},
        properties::WindowSize,
    },
    terminal::image::{
        Image,
        printer::{ImageRegistry, ImageSpec},
    },
    theme::raw::BackgroundImageFit,
};
use image::DynamicImage;
use std::{cell::RefCell, fmt, path::PathBuf, rc::Rc};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_image_from_path(
        &mut self,
        path: PathBuf,
        title: String,
        source_position: SourcePosition,
    ) -> BuildResult {
        let base_path = self.resource_base_path();
        let image = self.resources.image(&path, &base_path).map_err(|e| {
            self.invalid_presentation(source_position, InvalidPresentation::LoadImage { path, error: e.to_string() })
        })?;
        self.push_image(image, title, source_position)
    }

    pub(crate) fn push_image(&mut self, image: Image, title: String, source_position: SourcePosition) -> BuildResult {
        let attributes = self.parse_image_attributes(&title, &self.options.image_attribute_prefix, source_position)?;
        let size = match attributes.width {
            Some(percent) => ImageSize::WidthScaled { ratio: percent.as_ratio() },
            None => ImageSize::ShrinkIfNeeded,
        };
        let properties = ImageRenderProperties {
            size,
            background_color: self.theme.default_style.style.colors.background,
            ..Default::default()
        };
        self.chunk_operations.push(RenderOperation::RenderImage(image, properties));
        Ok(())
    }

    fn parse_image_attributes(
        &self,
        input: &str,
        attribute_prefix: &str,
        source_position: SourcePosition,
    ) -> Result<ImageAttributes, BuildError> {
        let mut attributes = ImageAttributes::default();
        for attribute in input.split(',') {
            let Some((prefix, suffix)) = attribute.split_once(attribute_prefix) else { continue };
            if !prefix.is_empty() || (attribute_prefix.is_empty() && suffix.is_empty()) {
                continue;
            }
            Self::parse_image_attribute(suffix, &mut attributes)
                .map_err(|e| self.invalid_presentation(source_position, e))?;
        }
        Ok(attributes)
    }

    fn parse_image_attribute(input: &str, attributes: &mut ImageAttributes) -> Result<(), ImageAttributeError> {
        let Some((key, value)) = input.split_once(':') else {
            return Err(ImageAttributeError::AttributeMissing);
        };
        match key {
            "width" | "w" => {
                let width = value.parse().map_err(ImageAttributeError::InvalidWidth)?;
                attributes.width = Some(width);
                Ok(())
            }
            _ => Err(ImageAttributeError::UnknownAttribute(key.to_string())),
        }
    }
}

pub(crate) struct CoverImageRenderer {
    source: DynamicImage,
    registry: ImageRegistry,
    z_index: i32,
    cache: RefCell<Option<(u16, u16, Image)>>,
}

impl CoverImageRenderer {
    fn new(source: DynamicImage, registry: ImageRegistry, z_index: i32) -> Self {
        Self { source, registry, z_index, cache: RefCell::new(None) }
    }

    fn crop_to_aspect(&self, dimensions: &WindowSize) -> DynamicImage {
        let px_per_col = if dimensions.width > 0 { dimensions.pixels_per_column() } else { 8.0 };
        let px_per_row = if dimensions.height > 0 { dimensions.pixels_per_row() } else { 16.0 };

        let screen_w = dimensions.columns as f64 * px_per_col;
        let screen_h = dimensions.rows as f64 * px_per_row;

        let img_w = self.source.width() as f64;
        let img_h = self.source.height() as f64;

        let screen_ratio = screen_w / screen_h;
        let img_ratio = img_w / img_h;

        let (crop_w, crop_h) = if img_ratio > screen_ratio {
            ((img_h * screen_ratio).round(), img_h)
        } else {
            (img_w, (img_w / screen_ratio).round())
        };

        let x = ((img_w - crop_w) / 2.0).round() as u32;
        let y = ((img_h - crop_h) / 2.0).round() as u32;
        self.source.crop_imm(x, y, crop_w as u32, crop_h as u32)
    }
}

impl fmt::Debug for CoverImageRenderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoverImageRenderer").finish()
    }
}

impl AsRenderOperations for CoverImageRenderer {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let cols = dimensions.columns;
        let rows = dimensions.rows;

        let mut cache = self.cache.borrow_mut();
        let image = match &*cache {
            Some((c, r, img)) if *c == cols && *r == rows => img.clone(),
            _ => {
                let cropped = self.crop_to_aspect(dimensions);
                let Ok(img) = self.registry.register(ImageSpec::Generated(cropped)) else {
                    return Vec::new();
                };
                *cache = Some((cols, rows, img.clone()));
                img
            }
        };

        vec![RenderOperation::RenderImage(
            image,
            ImageRenderProperties {
                z_index: self.z_index,
                size: ImageSize::Stretch,
                restore_cursor: true,
                background_color: None,
                position: ImagePosition::Cursor,
            },
        )]
    }
}

#[derive(Clone)]
pub(crate) struct BackgroundImageSlot {
    inner: Rc<RefCell<Option<BackgroundImageOp>>>,
}

enum BackgroundImageOp {
    Static(Image, ImageSize),
    Cover(CoverImageRenderer),
}

impl fmt::Debug for BackgroundImageOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(img, size) => f.debug_tuple("Static").field(img).field(size).finish(),
            Self::Cover(_) => f.write_str("Cover"),
        }
    }
}

impl BackgroundImageSlot {
    pub(crate) fn new() -> Self {
        Self { inner: Rc::new(RefCell::new(None)) }
    }

    pub(crate) fn set_static(&self, image: Image, fit: BackgroundImageFit) {
        *self.inner.borrow_mut() = Some(BackgroundImageOp::Static(image, fit.into()));
    }

    pub(crate) fn set_cover(&self, source: DynamicImage, registry: ImageRegistry) {
        let renderer = CoverImageRenderer::new(source, registry, -1);
        *self.inner.borrow_mut() = Some(BackgroundImageOp::Cover(renderer));
    }
}

impl fmt::Debug for BackgroundImageSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackgroundImageSlot").finish()
    }
}

impl AsRenderOperations for BackgroundImageSlot {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        match &*self.inner.borrow() {
            None => Vec::new(),
            Some(BackgroundImageOp::Static(image, size)) => {
                vec![RenderOperation::RenderImage(
                    image.clone(),
                    ImageRenderProperties {
                        z_index: -1,
                        size: size.clone(),
                        restore_cursor: true,
                        background_color: None,
                        position: ImagePosition::Cursor,
                    },
                )]
            }
            Some(BackgroundImageOp::Cover(renderer)) => renderer.as_render_operations(dimensions),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ImageAttributeError {
    #[error("invalid width: {0}")]
    InvalidWidth(PercentParseError),

    #[error("no attribute given")]
    AttributeMissing,

    #[error("unknown attribute: '{0}'")]
    UnknownAttribute(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ImageAttributes {
    width: Option<Percent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::builder::utils::Test;
    use rstest::rstest;

    #[rstest]
    #[case::width("image:width:50%", Some(50))]
    #[case::w("image:w:50%", Some(50))]
    #[case::nothing("", None)]
    #[case::no_prefix("width", None)]
    fn image_attributes(#[case] input: &str, #[case] expectation: Option<u8>) {
        let attributes = Test::new("").with_builder(|builder| {
            builder.parse_image_attributes(input, "image:", Default::default()).expect("failed to parse")
        });
        assert_eq!(attributes.width, expectation.map(Percent));
    }

    #[rstest]
    #[case::width("width:50%", Some(50))]
    #[case::empty("", None)]
    fn image_attributes_empty_prefix(#[case] input: &str, #[case] expectation: Option<u8>) {
        let attributes = Test::new("").with_builder(|builder| {
            builder.parse_image_attributes(input, "", Default::default()).expect("failed to parse")
        });
        assert_eq!(attributes.width, expectation.map(Percent));
    }
}
