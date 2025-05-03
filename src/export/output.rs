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
        image::printer::TerminalImage,
        virt::{TerminalGrid, VirtualTerminal},
    },
    tools::ThirdPartyTools,
};
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
    fn new(grid: TerminalGrid) -> Result<Self, ExportError> {
        let mut rows = Vec::new();
        rows.push(String::from("<div class=\"container\">"));
        for (y, row) in grid.rows.into_iter().enumerate() {
            let mut finalized_row = "<div class=\"content-line\"><pre>".to_string();
            let mut current_style = row.first().map(|c| c.style).unwrap_or_default();
            let mut current_string = String::new();
            let mut x = 0;
            while x < row.len() {
                let c = row[x];
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
                    let TerminalImage::Raw(raw_image) = image.image.image() else { panic!("not in raw image mode") };
                    let image_contents = raw_image.to_inline_html();
                    let width_pixels = (image.width_columns as f64 * FONT_SIZE as f64 * FONT_SIZE_WIDTH).ceil();
                    let image_tag = format!(
                        "<img width=\"{width_pixels}\" src=\"{image_contents}\" style=\"position: absolute\" />"
                    );
                    current_string.push_str(&image_tag);
                }
                x += c.style.size as usize;
            }
            if !current_string.is_empty() {
                finalized_row.push_str(&Self::finalize_string(&current_string, &current_style));
            }
            finalized_row.push_str("</pre></div>");
            rows.push(finalized_row);
        }
        rows.push(String::from("</div>"));

        Ok(HtmlSlide { rows, background_color: grid.background_color.as_ref().map(color_to_html) })
    }

    fn finalize_string(s: &str, style: &TextStyle) -> String {
        HtmlText::new(s, style, FontSize::Pixels(FONT_SIZE)).to_string()
    }
}

pub(crate) struct ContentManager {
    output_directory: OutputDirectory,
}

impl ContentManager {
    pub(crate) fn new(output_directory: OutputDirectory) -> Self {
        Self { output_directory }
    }

    fn persist_file(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.output_directory.path().join(name);
        fs::write(&path, data)?;
        Ok(path)
    }
}

pub(crate) enum OutputFormat {
    Pdf,
    Html,
}

pub(crate) struct ExportRenderer {
    content_manager: ContentManager,
    output_format: OutputFormat,
    dimensions: WindowSize,
    html_body: String,
    background_color: Option<String>,
}

impl ExportRenderer {
    pub(crate) fn new(dimensions: WindowSize, output_directory: OutputDirectory, output_type: OutputFormat) -> Self {
        let image_manager = ContentManager::new(output_directory);
        Self {
            content_manager: image_manager,
            dimensions,
            html_body: "".to_string(),
            background_color: None,
            output_format: output_type,
        }
    }

    pub(crate) fn process_slide(&mut self, slide: Slide) -> Result<(), ExportError> {
        let mut terminal = VirtualTerminal::new(self.dimensions.clone(), Default::default());
        let engine = RenderEngine::new(&mut terminal, self.dimensions.clone(), Default::default());
        engine.render(slide.iter_operations())?;

        let grid = terminal.into_contents();
        let slide = HtmlSlide::new(grid)?;
        if self.background_color.is_none() {
            self.background_color.clone_from(&slide.background_color);
        }
        for row in slide.rows {
            self.html_body.push_str(&row);
            self.html_body.push('\n');
        }
        Ok(())
    }

    pub(crate) fn generate(self, output_path: &Path) -> Result<(), ExportError> {
        let html_body = &self.html_body;
        let script = include_str!("script.js");
        let width = (self.dimensions.columns as f64 * FONT_SIZE as f64 * FONT_SIZE_WIDTH).ceil();
        let height = self.dimensions.rows * LINE_HEIGHT;
        let background_color = self.background_color.unwrap_or_else(|| "black".into());
        let container = match self.output_format {
            OutputFormat::Pdf => String::from("display: contents;"),
            OutputFormat::Html => String::from(
                "
                    width: 100%;
                    height: 100%;
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                ",
            ),
        };
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
            width: {width}px;
            height: {height}px;
            transform-origin: top left;
            background-color: {background_color};
        }}

        .container {{
            {container}
        }}

        .content-line {{
            line-height: {LINE_HEIGHT}px; 
            height: {LINE_HEIGHT}px;
            margin: 0px;
            width: {width}px;
        }}

        .hidden {{
            display: none;
        }}

        @page {{
            margin: 0;
            height: {height}px;
            width: {width}px;
        }}"
        );
        let html_script = match self.output_format {
            OutputFormat::Pdf => String::new(),
            OutputFormat::Html => {
                format!(
                    "
<script>
let originalWidth = {width};
let originalHeight = {height};
{script}
</script>"
                )
            }
        };
        let style = match self.output_format {
            OutputFormat::Pdf => String::new(),
            OutputFormat::Html => format!(
                "
<head>
<style>
{css}
</style>
</head>
                "
            ),
        };
        let html = format!(
            r"
<html>
{style}
<body>
{html_body}
{html_script}
</body>
</html>"
        );

        let html_path = self.content_manager.persist_file("index.html", html.as_bytes())?;
        let css_path = self.content_manager.persist_file("styles.css", css.as_bytes())?;

        match self.output_format {
            OutputFormat::Pdf => {
                ThirdPartyTools::weasyprint(&[
                    "-s",
                    css_path.to_string_lossy().as_ref(),
                    "--presentational-hints",
                    "-e",
                    "utf8",
                    html_path.to_string_lossy().as_ref(),
                    output_path.to_string_lossy().as_ref(),
                ])
                .run()?;
            }
            OutputFormat::Html => {
                fs::write(output_path, html.as_bytes())?;
            }
        }

        Ok(())
    }
}
