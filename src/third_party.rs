use crate::{
    custom::{default_mermaid_scale, default_typst_ppi},
    media::{image::Image, printer::RegisterImageError},
    style::Color,
    theme::{MermaidStyle, TypstStyle},
    tools::{ExecutionError, ThirdPartyTools},
    ImageRegistry,
};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, fs, io, path::Path};
use tempfile::tempdir_in;

const DEFAULT_HORIZONTAL_MARGIN: u16 = 5;
const DEFAULT_VERTICAL_MARGIN: u16 = 7;

pub struct ThirdPartyConfigs {
    pub typst_ppi: String,
    pub mermaid_scale: String,
}

pub struct ThirdPartyRender {
    config: ThirdPartyConfigs,
    image_registry: ImageRegistry,
    root_dir: String,
    cache: RefCell<HashMap<ImageSnippet<'static>, Image>>,
}

impl ThirdPartyRender {
    pub fn new(config: ThirdPartyConfigs, image_registry: ImageRegistry, root_dir: &Path) -> Self {
        // typst complains about empty paths so we give it a "." if we don't have one.
        let root_dir = match root_dir.to_string_lossy().to_string() {
            path if path.is_empty() => ".".into(),
            path => path,
        };
        Self { config, image_registry, root_dir, cache: Default::default() }
    }

    pub(crate) fn render_typst(&self, input: &str, style: &TypstStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: Cow::Borrowed(input), source: SnippetSource::Typst };
        if let Some(image) = self.cache.borrow().get(&snippet).cloned() {
            return Ok(image);
        }
        self.do_render_typst(snippet, input, style)
    }

    pub(crate) fn render_latex(&self, input: &str, style: &TypstStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: Cow::Borrowed(input), source: SnippetSource::Latex };
        if let Some(image) = self.cache.borrow().get(&snippet).cloned() {
            return Ok(image);
        }
        let output = ThirdPartyTools::pandoc(&["--from", "latex", "--to", "typst"])
            .stdin(input.as_bytes().into())
            .run_and_capture_stdout()?;

        let input = String::from_utf8_lossy(&output);
        self.do_render_typst(snippet, &input, style)
    }

    pub(crate) fn render_mermaid(&self, input: &str, style: &MermaidStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: Cow::Borrowed(input), source: SnippetSource::Mermaid };
        if let Some(image) = self.cache.borrow().get(&snippet).cloned() {
            return Ok(image);
        }
        let workdir = tempdir_in(&self.root_dir)?;
        let output_path = workdir.path().join("output.png");
        let input_path = workdir.path().join("input.mmd");
        fs::write(&input_path, input)?;

        ThirdPartyTools::mermaid(&[
            "-i",
            &input_path.to_string_lossy(),
            "-o",
            &output_path.to_string_lossy(),
            "-s",
            &self.config.mermaid_scale,
            "-t",
            style.theme.as_deref().unwrap_or("default"),
            "-b",
            style.background.as_deref().unwrap_or("white"),
        ])
        .run()?;

        self.load_image(snippet, &output_path)
    }

    fn do_render_typst(
        &self,
        snippet: ImageSnippet,
        input: &str,
        style: &TypstStyle,
    ) -> Result<Image, ThirdPartyRenderError> {
        let workdir = tempdir_in(&self.root_dir)?;
        let mut typst_input = Self::generate_page_header(style)?;
        typst_input.push_str(input);

        let input_path = workdir.path().join("input.typst");
        fs::write(&input_path, &typst_input)?;

        let output_path = workdir.path().join("output.png");
        ThirdPartyTools::typst(&[
            "compile",
            "--format",
            "png",
            "--root",
            &self.root_dir,
            "--ppi",
            &self.config.typst_ppi,
            &input_path.to_string_lossy(),
            &output_path.to_string_lossy(),
        ])
        .run()?;

        self.load_image(snippet, &output_path)
    }

    fn generate_page_header(style: &TypstStyle) -> Result<String, ThirdPartyRenderError> {
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

    fn as_typst_color(color: &Color) -> Result<String, ThirdPartyRenderError> {
        match color.as_rgb() {
            Some((r, g, b)) => Ok(format!("rgb(\"#{r:02x}{g:02x}{b:02x}\")")),
            None => Err(ThirdPartyRenderError::UnsupportedColor(color.to_string())),
        }
    }

    fn load_image(&self, snippet: ImageSnippet, path: &Path) -> Result<Image, ThirdPartyRenderError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        let image = self.image_registry.register_image(image)?;
        let snippet = ImageSnippet { snippet: Cow::Owned(snippet.snippet.into_owned()), source: snippet.source };
        self.cache.borrow_mut().insert(snippet, image.clone());
        Ok(image)
    }
}

impl Default for ThirdPartyRender {
    fn default() -> Self {
        let config = ThirdPartyConfigs {
            typst_ppi: default_typst_ppi().to_string(),
            mermaid_scale: default_mermaid_scale().to_string(),
        };
        Self::new(config, Default::default(), Path::new("."))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThirdPartyRenderError {
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

#[derive(Hash, PartialEq, Eq)]
enum SnippetSource {
    Typst,
    Latex,
    Mermaid,
}

#[derive(Hash, PartialEq, Eq)]
struct ImageSnippet<'a> {
    snippet: Cow<'a, str>,
    source: SnippetSource,
}
