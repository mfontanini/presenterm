use crate::{
    render::media::{Image, ImageSource, InvalidImage},
    style::Color,
    theme::TypstStyle,
};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;

const DEFAULT_PPI: u32 = 300;
const DEFAULT_HORIZONTAL_MARGIN: u16 = 5;
const DEFAULT_VERTICAL_MARGIN: u16 = 7;

pub struct TypstRender {
    ppi: String,
}

impl TypstRender {
    pub fn new(ppi: u32) -> Self {
        Self { ppi: ppi.to_string() }
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
        let mut child = Command::new("pandoc")
            .args(["--from", "latex", "--to", "typst"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TypstRenderError::CommandRun("pandoc", e.to_string()))?;

        child.stdin.take().expect("no stdin").write_all(input.as_bytes())?;
        let output = child.wait_with_output().map_err(|e| TypstRenderError::CommandRun("pandoc", e.to_string()))?;
        Self::validate_output(&output, "pandoc")?;

        let input = String::from_utf8_lossy(&output.stdout);
        self.render_typst(&input, style)
    }

    fn render_to_image(&self, base_path: &Path, path: &Path) -> Result<Image, TypstRenderError> {
        let output_path = base_path.join("output.png");
        let output = Command::new("typst")
            .args([
                "compile",
                "--format",
                "png",
                "--ppi",
                &self.ppi,
                &path.to_string_lossy(),
                &output_path.to_string_lossy(),
            ])
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| TypstRenderError::CommandRun("typst", e.to_string()))?;
        Self::validate_output(&output, "typst")?;

        let png_contents = fs::read(&output_path)?;
        let image = Image::new(&png_contents, ImageSource::Generated)?;
        Ok(image)
    }

    fn validate_output(output: &Output, name: &'static str) -> Result<(), TypstRenderError> {
        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            let error = error.lines().take(10).collect();
            Err(TypstRenderError::CommandRun(name, error))
        }
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
        Self::new(DEFAULT_PPI)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TypstRenderError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid output image: {0}")]
    InvalidImage(#[from] InvalidImage),

    #[error("running command '{0}': {1}")]
    CommandRun(&'static str, String),

    #[error("unsupported color '{0}', only RGB is supported")]
    UnsupportedColor(String),
}
