use crate::{
    markdown::elements::{Percent, PercentParseError, SourcePosition},
    presentation::builder::{
        BuildResult, PresentationBuilder,
        error::{BuildError, InvalidPresentation},
    },
    render::operation::{ImageRenderProperties, ImageSize, RenderOperation},
    terminal::image::Image,
};
use std::path::PathBuf;

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
