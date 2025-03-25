use crate::{
    MarkdownParser, Resources,
    code::execute::SnippetExecutor,
    config::KeyBindingsConfig,
    export::pdf::PdfRender,
    markdown::{parse::ParseError, text_style::Color},
    presentation::{
        Presentation, Slide,
        builder::{BuildError, PresentationBuilder, PresentationBuilderOptions, Themes},
    },
    render::{
        RenderError,
        operation::{AsRenderOperations, RenderAsyncState, RenderOperation},
        properties::WindowSize,
    },
    theme::{ProcessingThemeError, raw::PresentationTheme},
    third_party::ThirdPartyRender,
    tools::{ExecutionError, ThirdPartyTools},
};
use crossterm::{
    cursor::{MoveToColumn, MoveToNextLine, MoveUp},
    execute,
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};
use image::ImageError;
use std::{
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    thread::sleep,
    time::Duration,
};
use tempfile::TempDir;

pub enum OutputDirectory {
    Temporary(TempDir),
    External(PathBuf),
}

impl OutputDirectory {
    pub fn temporary() -> io::Result<Self> {
        let dir = TempDir::with_suffix("presenterm")?;
        Ok(Self::Temporary(dir))
    }

    pub fn external(path: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&path)?;
        Ok(Self::External(path))
    }

    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Temporary(temp) => temp.path(),
            Self::External(path) => path,
        }
    }
}

/// Allows exporting presentations into PDF.
pub struct Exporter<'a> {
    parser: MarkdownParser<'a>,
    default_theme: &'a PresentationTheme,
    resources: Resources,
    third_party: ThirdPartyRender,
    code_executor: Rc<SnippetExecutor>,
    themes: Themes,
    dimensions: WindowSize,
    options: PresentationBuilderOptions,
}

impl<'a> Exporter<'a> {
    /// Construct a new exporter.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parser: MarkdownParser<'a>,
        default_theme: &'a PresentationTheme,
        resources: Resources,
        third_party: ThirdPartyRender,
        code_executor: Rc<SnippetExecutor>,
        themes: Themes,
        mut options: PresentationBuilderOptions,
        mut dimensions: WindowSize,
    ) -> Self {
        // We don't want dynamically highlighted code blocks.
        options.allow_mutations = false;
        options.theme_options.font_size_supported = true;

        // Make sure we have a 1:2 aspect ratio.
        let width = (0.5 * dimensions.columns as f64) / (dimensions.rows as f64 / dimensions.height as f64);
        dimensions.width = width as u16;

        Self { parser, default_theme, resources, third_party, code_executor, themes, options, dimensions }
    }

    /// Export the given presentation into PDF.
    ///
    /// This uses a separate `presenterm-export` tool.
    pub fn export_pdf(
        mut self,
        presentation_path: &Path,
        output_directory: OutputDirectory,
        output_path: Option<&Path>,
    ) -> Result<(), ExportError> {
        println!(
            "exporting using rows={}, columns={}, width={}, height={}",
            self.dimensions.rows, self.dimensions.columns, self.dimensions.width, self.dimensions.height
        );

        println!("checking for weasyprint...");
        Self::validate_weasyprint_exists()?;
        Self::log("weasyprint installation found")?;

        let content = fs::read_to_string(presentation_path).map_err(ExportError::ReadPresentation)?;
        let elements = self.parser.parse(&content)?;

        let mut presentation = PresentationBuilder::new(
            self.default_theme,
            self.resources.clone(),
            &mut self.third_party,
            self.code_executor.clone(),
            &self.themes,
            Default::default(),
            KeyBindingsConfig::default(),
            self.options.clone(),
        )?
        .build(elements)?;
        Self::validate_theme_colors(&presentation)?;

        let mut render = PdfRender::new(self.dimensions, output_directory);
        Self::log("waiting for images to be generated and code to be executed, if any...")?;
        for slide in presentation.iter_slides_mut() {
            Self::render_async_images(slide);
        }
        for (index, slide) in presentation.into_slides().into_iter().enumerate() {
            let index = index + 1;
            Self::log(&format!("processing slide {index}..."))?;
            render.process_slide(slide)?;
        }
        Self::log("invoking weasyprint...")?;

        let pdf_path = match output_path {
            Some(path) => path.to_path_buf(),
            None => presentation_path.with_extension("pdf"),
        };
        render.generate(&pdf_path)?;

        execute!(
            io::stdout(),
            PrintStyledContent(
                format!("output file is at {}\n", pdf_path.display()).stylize().with(Color::Green.into())
            )
        )?;
        Ok(())
    }

    fn render_async_images(slide: &mut Slide) {
        for op in slide.iter_operations_mut() {
            if let RenderOperation::RenderAsync(inner) = op {
                loop {
                    match inner.poll_state() {
                        RenderAsyncState::Rendering { .. } => {
                            sleep(Duration::from_millis(200));
                            continue;
                        }
                        RenderAsyncState::Rendered | RenderAsyncState::JustFinishedRendering => break,
                        RenderAsyncState::NotStarted => inner.start_render(),
                    };
                }
                let window_size = WindowSize { rows: 0, columns: 0, width: 0, height: 0 };
                let new_operations = inner.as_render_operations(&window_size);
                // Replace this operation with a new operation that contains the replaced image
                // and any other unmodified operations.
                *op = RenderOperation::RenderDynamic(Rc::new(RenderMany(new_operations)));
            }
        }
    }

    fn validate_weasyprint_exists() -> Result<(), ExportError> {
        let result = ThirdPartyTools::weasyprint(&["--version"]).run_and_capture_stdout();
        match result {
            Ok(_) => Ok(()),
            Err(ExecutionError::Execution { .. }) => Err(ExportError::WeasyprintMissing),
            Err(e) => Err(e.into()),
        }
    }

    fn validate_theme_colors(presentation: &Presentation) -> Result<(), ExportError> {
        for slide in presentation.iter_slides() {
            for operation in slide.iter_visible_operations() {
                let RenderOperation::SetColors(colors) = operation else {
                    continue;
                };
                // The PDF requires a specific theme to be set, as "no background" means "what the
                // browser uses" which is likely white and it will probably look terrible. It's
                // better to err early and let you choose a theme that contains _some_ color.
                if colors.background.is_none() {
                    return Err(ExportError::UnsupportedColor("background"));
                }
                if colors.foreground.is_none() {
                    return Err(ExportError::UnsupportedColor("foreground"));
                }
            }
        }
        Ok(())
    }

    fn log(text: &str) -> io::Result<()> {
        execute!(
            io::stdout(),
            MoveUp(1),
            Clear(ClearType::CurrentLine),
            MoveToColumn(0),
            Print(text),
            MoveToNextLine(1)
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExportError {
    #[error("failed to read presentation: {0}")]
    ReadPresentation(io::Error),

    #[error("failed to parse presentation: {0}")]
    ParsePresentation(#[from] ParseError),

    #[error("failed to build presentation: {0}")]
    BuildPresentation(#[from] BuildError),

    #[error("unsupported {0} color in theme")]
    UnsupportedColor(&'static str),

    #[error("generating images: {0}")]
    GeneratingImages(#[from] ImageError),

    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error("weasyprint not found")]
    WeasyprintMissing,

    #[error("processing theme: {0}")]
    ProcessingTheme(#[from] ProcessingThemeError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("render: {0}")]
    Render(#[from] RenderError),
}

#[derive(Debug)]
struct RenderMany(Vec<RenderOperation>);

impl AsRenderOperations for RenderMany {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        self.0.clone()
    }
}
