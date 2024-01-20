use crate::{
    media::{
        image::{Image, ImageSource},
        printer::{PrintImage, RegisterImageError},
    },
    style::Color,
    theme::TypstStyle,
    tools::{ExecutionError, ThirdPartyTools},
    ImagePrinter,
};
use std::{
    fs,
    io::{self},
    path::Path,
    rc::Rc,
};
use tempfile::tempdir;

const DEFAULT_PPI: u32 = 300;
const DEFAULT_HORIZONTAL_MARGIN: u16 = 5;
const DEFAULT_VERTICAL_MARGIN: u16 = 7;

pub struct TypstRender {
    ppi: String,
    printer: Rc<ImagePrinter>,
}

impl TypstRender {
    pub fn new(ppi: u32, printer: Rc<ImagePrinter>) -> Self {
        Self { ppi: ppi.to_string(), printer }
    }

    pub(crate) fn render_typst(&self, input: &str, style: &TypstStyle) -> Result<Image, TypstRenderError> {
        let workdir = tempdir()?;
        let mut typst_input = Self::generate_page_header(style)?;
        typst_input.push_str(input);

        let input_path = workdir.path().join("input.typst");
        fs::write(&input_path, &typst_input)?;
        self.render_to_image(workdir.path(), &input_path)
    }

    pub(crate) fn render_latex(&self, input: &str, style: &TypstStyle) -> Result<Image, TypstRenderError> {
        let output = ThirdPartyTools::pandoc(&["--from", "latex", "--to", "typst"])
            .stdin(input.as_bytes().into())
            .run_and_capture_stdout()?;

        let input = String::from_utf8_lossy(&output);
        self.render_typst(&input, style)
    }

    fn render_to_image(&self, base_path: &Path, path: &Path) -> Result<Image, TypstRenderError> {
        let output_path = base_path.join("output.png");
        ThirdPartyTools::typst(&[
            "compile",
            "--format",
            "png",
            "--ppi",
            &self.ppi,
            &path.to_string_lossy(),
            &output_path.to_string_lossy(),
        ])
        .run()?;

        let png_contents = fs::read(&output_path)?;
        let image = image::load_from_memory(&png_contents)?;
        let resource = self.printer.register_image(image)?;
        let image = Image::new(resource, ImageSource::Generated);
        Ok(image)
    }

    fn generate_page_header(style: &TypstStyle) -> Result<String, TypstRenderError> {
        let x_margin = style.horizontal_margin.unwrap_or(DEFAULT_HORIZONTAL_MARGIN);
        let y_margin = style.vertical_margin.unwrap_or(DEFAULT_VERTICAL_MARGIN);
        let background =
            style.colors.background.as_ref().map(Self::as_typst_color).unwrap_or_else(|| Ok(String::from("none")))?;
        let mut header = format!(
            "#set page(width: auto, height: auto, margin: (x: {x_margin}pt, y: {y_margin}pt), fill: {background})\n"
        );
        if let Some(color) = &style.colors.foreground {
            let color = Self::as_typst_color(color)?;
            header.push_str(&format!("#set text(fill: {color})\n"));
        }
        Ok(header)
    }

    fn as_typst_color(color: &Color) -> Result<String, TypstRenderError> {
        match color.as_rgb() {
            Some((r, g, b)) => Ok(format!("rgb(\"#{r:02x}{g:02x}{b:02x}\")")),
            None => Err(TypstRenderError::UnsupportedColor(color.to_string())),
        }
    }
}

impl Default for TypstRender {
    fn default() -> Self {
        Self::new(DEFAULT_PPI, Default::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TypstRenderError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid image: {0}")]
    InvalidImage(#[from] image::ImageError),

    #[error("invalid image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("unsupported color '{0}', only RGB is supported")]
    UnsupportedColor(String),
}
