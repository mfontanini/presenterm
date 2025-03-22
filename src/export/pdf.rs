use super::{
    exporter::{ExportError, OutputDirectory},
    html::{FontSize, color_to_html},
};
use crate::{
    export::html::HtmlText,
    markdown::text_style::TextStyle,
    presentation::Slide,
    render::{engine::RenderEngine, properties::WindowSize},
    terminal::{
        image::{
            Image, ImageSource,
            printer::{ImageProperties, TerminalImage},
        },
        virt::{TerminalGrid, VirtualTerminal},
    },
    tools::ThirdPartyTools,
};
use image::{ImageEncoder, codecs::png::PngEncoder};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

// A magical multiplier that converts a font size in pixels to a font width.
//
// There's probably something somewhere that specifies what the relationship
// really is but I found this by trial and error an I'm okay with that.
const FONT_SIZE_WIDTH: f64 = 0.605;

const FONT_SIZE: u16 = 10;
const LINE_HEIGHT: u16 = 12;

struct HtmlSlide {
    rows: Vec<String>,
    background_color: Option<String>,
}

impl HtmlSlide {
    fn new(grid: TerminalGrid, content_manager: &mut ContentManager) -> Result<Self, ExportError> {
        let mut rows = Vec::new();
        for (y, row) in grid.rows.into_iter().enumerate() {
            let mut finalized_row = "<div class=\"content-line\"><pre>".to_string();
            let mut current_style = row.first().map(|c| c.style).unwrap_or_default();
            let mut current_string = String::new();
            for (x, c) in row.into_iter().enumerate() {
                if c.style != current_style {
                    finalized_row.push_str(&Self::finalize_string(&current_string, &current_style));
                    current_string = String::new();
                    current_style = c.style;
                }
                match c.character {
                    '<' => current_string.push_str("&lt;"),
                    '>' => current_string.push_str("&gt;"),
                    other => current_string.push(other),
                }
                if let Some(image) = grid.images.get(&(y as u16, x as u16)) {
                    let image_path = content_manager.persist_image(&image.image)?;
                    let image_path_str = image_path.display();
                    let width_pixels = (image.width_columns as f64 * FONT_SIZE as f64 * FONT_SIZE_WIDTH).ceil();
                    let image_tag = format!(
                        "<img width=\"{width_pixels}\" src=\"file://{image_path_str}\" style=\"position: absolute\" />"
                    );
                    current_string.push_str(&image_tag);
                }
            }
            if !current_string.is_empty() {
                finalized_row.push_str(&Self::finalize_string(&current_string, &current_style));
            }
            finalized_row.push_str("</pre></div>");
            rows.push(finalized_row);
        }

        Ok(HtmlSlide { rows, background_color: grid.background_color.as_ref().map(color_to_html) })
    }

    fn finalize_string(s: &str, style: &TextStyle) -> String {
        HtmlText::new(s, style, FontSize::Pixels(FONT_SIZE)).to_string()
    }
}

pub(crate) struct ContentManager {
    output_directory: OutputDirectory,
    image_count: usize,
}

impl ContentManager {
    pub(crate) fn new(output_directory: OutputDirectory) -> Self {
        Self { output_directory, image_count: 0 }
    }

    fn persist_image(&mut self, image: &Image) -> Result<PathBuf, ExportError> {
        match image.source.clone() {
            ImageSource::Filesystem(path) => Ok(path),
            ImageSource::Generated => {
                let mut buffer = Vec::new();
                let dimensions = image.dimensions();
                let TerminalImage::Ascii(resource) = image.image.as_ref() else { panic!("not in ascii mode") };
                PngEncoder::new(&mut buffer).write_image(
                    resource.as_bytes(),
                    dimensions.0,
                    dimensions.1,
                    resource.color().into(),
                )?;
                let name = format!("img-{}.png", self.image_count);
                let path = self.output_directory.path().join(name);
                fs::write(&path, buffer)?;
                self.image_count += 1;
                Ok(path)
            }
        }
    }

    fn persist_file(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.output_directory.path().join(name);
        fs::write(&path, data)?;
        Ok(path)
    }
}

pub(crate) struct PdfRender {
    content_manager: ContentManager,
    dimensions: WindowSize,
    html_body: String,
    background_color: Option<String>,
}

impl PdfRender {
    pub(crate) fn new(dimensions: WindowSize, output_directory: OutputDirectory) -> Self {
        let image_manager = ContentManager::new(output_directory);
        Self { content_manager: image_manager, dimensions, html_body: "".to_string(), background_color: None }
    }

    pub(crate) fn process_slide(&mut self, slide: Slide) -> Result<(), ExportError> {
        let mut terminal = VirtualTerminal::new(self.dimensions.clone());
        let engine = RenderEngine::new(&mut terminal, self.dimensions.clone(), Default::default());
        engine.render(slide.iter_operations())?;

        let grid = terminal.into_contents();
        let slide = HtmlSlide::new(grid, &mut self.content_manager)?;
        if self.background_color.is_none() {
            self.background_color.clone_from(&slide.background_color);
        }
        for row in slide.rows {
            self.html_body.push_str(&row);
            self.html_body.push('\n');
        }
        Ok(())
    }

    pub(crate) fn generate(self, pdf_path: &Path) -> Result<(), ExportError> {
        let html_body = &self.html_body;
        let html = format!(
            r#"<html>
<head>
</head>
<body>
{html_body}</body>
</html>"#
        );
        let width = (self.dimensions.columns as f64 * FONT_SIZE as f64 * FONT_SIZE_WIDTH).ceil();
        let height = self.dimensions.rows * LINE_HEIGHT;
        let background_color = self.background_color.unwrap_or_else(|| "black".into());
        let css = format!(
            r"
        pre {{
            margin: 0;
            padding: 0;
        }}

        span {{
            display: inline-block;
        }}

        body {{
            margin: 0;
            font-size: {FONT_SIZE}px;
            line-height: {LINE_HEIGHT}px;
            background-color: {background_color};
            width: {width}px;
        }}

        .content-line {{
            line-height: {LINE_HEIGHT}px; 
            height: {LINE_HEIGHT}px;
            margin: 0px;
            width: {width}px;
        }}

        @page {{
            margin: 0;
            height: {height}px;
            width: {width}px;
        }}"
        );

        let html_path = self.content_manager.persist_file("index.html", html.as_bytes())?;
        let css_path = self.content_manager.persist_file("styles.css", css.as_bytes())?;
        ThirdPartyTools::weasyprint(&[
            "-s",
            css_path.to_string_lossy().as_ref(),
            "--presentational-hints",
            "-e",
            "utf8",
            html_path.to_string_lossy().as_ref(),
            pdf_path.to_string_lossy().as_ref(),
        ])
        .run()?;
        Ok(())
    }
}
