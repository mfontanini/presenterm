use crate::{
    MarkdownParser, PresentationTheme, Resources,
    custom::KeyBindingsConfig,
    execute::SnippetExecutor,
    markdown::parse::ParseError,
    presentation::{AsRenderOperations, Presentation, RenderAsyncState, RenderOperation},
    processing::builder::{BuildError, PresentationBuilder, PresentationBuilderOptions, Themes},
    render::properties::WindowSize,
    terminal::image::{
        Image, ImageSource,
        printer::{ImageProperties, TerminalImage},
    },
    third_party::ThirdPartyRender,
    tools::{ExecutionError, ThirdPartyTools},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageEncoder, ImageError, codecs::png::PngEncoder};
use semver::Version;
use serde::Serialize;
use std::{
    env, fs, io, iter,
    path::{Path, PathBuf},
    rc::Rc,
    thread::sleep,
    time::Duration,
};

const MINIMUM_EXPORTER_VERSION: Version = Version::new(0, 2, 0);
const ASYNC_RENDER_WAIT_COUNT: usize = 8;

/// Allows exporting presentations into PDF.
pub struct Exporter<'a> {
    parser: MarkdownParser<'a>,
    default_theme: &'a PresentationTheme,
    resources: Resources,
    third_party: ThirdPartyRender,
    code_executor: Rc<SnippetExecutor>,
    themes: Themes,
    options: PresentationBuilderOptions,
}

impl<'a> Exporter<'a> {
    /// Construct a new exporter.
    pub fn new(
        parser: MarkdownParser<'a>,
        default_theme: &'a PresentationTheme,
        resources: Resources,
        third_party: ThirdPartyRender,
        code_executor: Rc<SnippetExecutor>,
        themes: Themes,
        options: PresentationBuilderOptions,
    ) -> Self {
        Self { parser, default_theme, resources, third_party, code_executor, themes, options }
    }

    /// Export the given presentation into PDF.
    ///
    /// This uses a separate `presenterm-export` tool.
    pub fn export_pdf(&mut self, presentation_path: &Path, extra_args: &[&str]) -> Result<(), ExportError> {
        Self::validate_exporter_version()?;

        println!("Analyzing presentation...");
        let metadata = self.generate_metadata(presentation_path)?;
        println!("Invoking presenterm-export...");
        Self::execute_exporter(metadata, extra_args)?;
        Ok(())
    }

    /// Generate the metadata for the given presentation.
    pub fn generate_metadata(&mut self, presentation_path: &Path) -> Result<ExportMetadata, ExportError> {
        let content = fs::read_to_string(presentation_path).map_err(ExportError::ReadPresentation)?;
        let metadata = self.extract_metadata(&content, presentation_path)?;
        Ok(metadata)
    }

    fn validate_exporter_version() -> Result<(), ExportError> {
        let result = ThirdPartyTools::presenterm_export(&["--version"]).run_and_capture_stdout();
        let version = match result {
            Ok(version) => String::from_utf8(version).expect("not utf8"),
            Err(ExecutionError::Execution { .. }) => return Err(ExportError::MinimumVersion),
            Err(e) => return Err(e.into()),
        };
        let version = Version::parse(version.trim()).map_err(|_| ExportError::MinimumVersion)?;
        if version >= MINIMUM_EXPORTER_VERSION { Ok(()) } else { Err(ExportError::MinimumVersion) }
    }

    /// Extract the metadata necessary to make an export.
    fn extract_metadata(&mut self, content: &str, path: &Path) -> Result<ExportMetadata, ExportError> {
        let elements = self.parser.parse(content)?;
        let path = path.canonicalize().expect("canonicalize");
        let mut presentation = PresentationBuilder::new(
            self.default_theme,
            &mut self.resources,
            &mut self.third_party,
            self.code_executor.clone(),
            &self.themes,
            Default::default(),
            KeyBindingsConfig::default(),
            self.options.clone(),
        )
        .build(elements)?;

        let async_renders = Self::count_async_render_operations(&presentation);
        let images = Self::build_image_metadata(&mut presentation)?;
        Self::validate_theme_colors(&presentation)?;
        let commands = Self::build_capture_commands(presentation, async_renders);
        let metadata = ExportMetadata { commands, presentation_path: path, images };
        Ok(metadata)
    }

    fn execute_exporter(metadata: ExportMetadata, extra_args: &[&str]) -> Result<(), ExportError> {
        let presenterm_path = env::current_exe().map_err(ExportError::Io)?;
        let presenterm_path = presenterm_path.display().to_string();
        let presentation_path = metadata.presentation_path.display().to_string();
        let metadata = serde_json::to_vec(&metadata).expect("serialization failed");
        let mut args = vec![&presenterm_path, "--enable-export-mode"];
        args.extend(extra_args);
        args.push(&presentation_path);
        ThirdPartyTools::presenterm_export(&args).stdin(metadata).run()?;
        Ok(())
    }

    fn build_capture_commands(mut presentation: Presentation, async_renders: usize) -> Vec<CaptureCommand> {
        let mut commands = Vec::new();
        let slide_chunks: Vec<_> = presentation.iter_slides().map(|slide| slide.iter_chunks().count()).collect();
        let mut next_slide = |commands: &mut Vec<CaptureCommand>| {
            commands.push(CaptureCommand::SendKeys { keys: "l" });
            commands.push(CaptureCommand::WaitForChange);
            presentation.jump_next();
        };
        commands.extend(iter::repeat(CaptureCommand::WaitForChange).take(ASYNC_RENDER_WAIT_COUNT * async_renders));
        for chunks in slide_chunks {
            for _ in 0..chunks - 1 {
                next_slide(&mut commands);
            }
            commands.push(CaptureCommand::Capture);
            next_slide(&mut commands);
        }
        commands
    }

    fn count_async_render_operations(presentation: &Presentation) -> usize {
        presentation
            .iter_slides()
            .map(|slide| {
                slide.iter_visible_operations().filter(|op| matches!(op, RenderOperation::RenderAsync(_))).count()
            })
            .sum()
    }

    fn build_image_metadata(presentation: &mut Presentation) -> Result<Vec<ImageMetadata>, ExportError> {
        let mut replacer = ImageReplacer::default();
        replacer.replace_presentation_images(presentation);

        let mut positions = Vec::new();
        for image in replacer.images {
            let meta = match image.original.source {
                ImageSource::Filesystem(path) => {
                    let path = Some(path.canonicalize().map_err(ExportError::Io)?);
                    ImageMetadata { path, color: image.color, contents: None }
                }
                ImageSource::Generated => {
                    let mut buffer = Vec::new();
                    let dimensions = image.original.dimensions();
                    let TerminalImage::Ascii(resource) = image.original.image.as_ref() else {
                        panic!("not in ascii mode")
                    };
                    PngEncoder::new(&mut buffer).write_image(
                        resource.as_bytes(),
                        dimensions.0,
                        dimensions.1,
                        resource.color().into(),
                    )?;
                    let contents = Some(STANDARD.encode(buffer));
                    ImageMetadata { path: None, color: image.color, contents }
                }
            };
            positions.push(meta);
        }
        Ok(positions)
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

    #[error("minimum presenterm-export version ({MINIMUM_EXPORTER_VERSION}) not met")]
    MinimumVersion,

    #[error("io: {0}")]
    Io(io::Error),
}

/// The metadata necessary to export a presentation.
#[derive(Clone, Debug, Serialize)]
pub struct ExportMetadata {
    presentation_path: PathBuf,
    images: Vec<ImageMetadata>,
    commands: Vec<CaptureCommand>,
}

/// Metadata about an image.
#[derive(Clone, Debug, Serialize)]
struct ImageMetadata {
    path: Option<PathBuf>,
    contents: Option<String>,
    color: u32,
}

/// A command to whoever is capturing us indicating what to do.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum CaptureCommand {
    Capture,
    SendKeys { keys: &'static str },
    WaitForChange,
}

struct ReplacedImage {
    original: Image,
    color: u32,
}

pub(crate) struct ImageReplacer {
    next_color: u32,
    images: Vec<ReplacedImage>,
}

impl ImageReplacer {
    pub(crate) fn replace_presentation_images(&mut self, presentation: &mut Presentation) {
        let callback = |operation: &mut RenderOperation| {
            match operation {
                RenderOperation::RenderImage(image, properties) => {
                    let replacement = self.replace_image(image.clone());
                    *operation = RenderOperation::RenderImage(replacement, properties.clone());
                }
                RenderOperation::RenderAsync(inner) => {
                    loop {
                        match inner.poll_state() {
                            RenderAsyncState::NotStarted => return,
                            RenderAsyncState::Rendering { .. } => {
                                sleep(Duration::from_millis(200));
                                continue;
                            }
                            RenderAsyncState::Rendered | RenderAsyncState::JustFinishedRendering => break,
                        };
                    }

                    let window_size = WindowSize { rows: 0, columns: 0, width: 0, height: 0 };
                    let mut new_operations = Vec::new();
                    for operation in inner.as_render_operations(&window_size) {
                        if let RenderOperation::RenderImage(image, properties) = operation {
                            let image = self.replace_image(image);
                            new_operations.push(RenderOperation::RenderImage(image, properties));
                        } else {
                            new_operations.push(operation);
                        }
                    }
                    // Replace this operation with a new operation that contains the replaced image
                    // and any other unmodified operations.
                    *operation = RenderOperation::RenderDynamic(Rc::new(RenderMany(new_operations)));
                }
                _ => (),
            };
        };

        presentation.mutate_operations(callback);
    }

    fn replace_image(&mut self, image: Image) -> Image {
        let dimensions = image.dimensions();
        let color = self.allocate_color();
        let rgb_color = Self::as_rgb(color);

        let mut replacement = DynamicImage::new_rgb8(dimensions.0, dimensions.1);
        let buffer = replacement.as_mut_rgb8().expect("not rgb8");
        for pixel in buffer.pixels_mut() {
            pixel.0 = rgb_color;
        }
        self.images.push(ReplacedImage { original: image, color });

        Image::new(TerminalImage::Ascii(replacement.into()), ImageSource::Generated)
    }

    fn allocate_color(&mut self) -> u32 {
        let color = self.next_color;
        self.next_color += 1;
        color
    }

    fn as_rgb(color: u32) -> [u8; 3] {
        [(color >> 16) as u8, (color >> 8) as u8, (color & 0xff) as u8]
    }
}

impl Default for ImageReplacer {
    fn default() -> Self {
        Self { next_color: 0xffbad3, images: Vec::new() }
    }
}

#[derive(Debug)]
struct RenderMany(Vec<RenderOperation>);

impl AsRenderOperations for RenderMany {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        self.0.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::theme::PresentationThemeSet;
    use comrak::Arena;

    fn extract_metadata(content: &str, path: &str) -> ExportMetadata {
        let arena = Arena::new();
        let parser = MarkdownParser::new(&arena);
        let theme = PresentationThemeSet::default().load_by_name("dark").unwrap();
        let resources = Resources::new("examples", Default::default());
        let third_party = ThirdPartyRender::default();
        let code_executor = Default::default();
        let themes = Themes::default();
        let options = PresentationBuilderOptions { allow_mutations: false, ..Default::default() };
        let mut exporter = Exporter::new(parser, &theme, resources, third_party, code_executor, themes, options);
        exporter.extract_metadata(content, Path::new(path)).expect("metadata extraction failed")
    }

    #[test]
    fn metadata() {
        let presentation = r"
First

<!-- end_slide -->

hi
<!-- pause -->
mom

<!-- end_slide -->

![](doge.png)

<!-- end_slide -->

bye
<!-- pause -->
mom
        ";

        let meta = extract_metadata(presentation, "examples/demo.md");

        use CaptureCommand::*;
        let expected_commands = vec![
            // First slide
            Capture,
            SendKeys { keys: "l" },
            WaitForChange,
            // Second slide...
            SendKeys { keys: "l" },
            WaitForChange,
            Capture,
            SendKeys { keys: "l" },
            WaitForChange,
            // Third slide...
            Capture,
            SendKeys { keys: "l" },
            WaitForChange,
            // Fourth slide...
            SendKeys { keys: "l" },
            WaitForChange,
            Capture,
            SendKeys { keys: "l" },
            WaitForChange,
        ];
        assert_eq!(meta.commands, expected_commands);
    }
}
