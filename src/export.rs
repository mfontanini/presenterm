use crate::{
    builder::{BuildError, PresentationBuilder, PresentationBuilderOptions, Themes},
    markdown::{elements::MarkdownElement, parse::ParseError},
    presentation::{Presentation, RenderOperation},
    typst::TypstRender,
    CodeHighlighter, MarkdownParser, PresentationTheme, Resources,
};
use serde::Serialize;
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const COMMAND: &str = "presenterm-export";

/// Allows exporting presentations into PDF.
pub struct Exporter<'a> {
    parser: MarkdownParser<'a>,
    default_theme: &'a PresentationTheme,
    resources: Resources,
    typst: TypstRender,
    themes: Themes,
}

impl<'a> Exporter<'a> {
    /// Construct a new exporter.
    pub fn new(
        parser: MarkdownParser<'a>,
        default_theme: &'a PresentationTheme,
        resources: Resources,
        typst: TypstRender,
        themes: Themes,
    ) -> Self {
        Self { parser, default_theme, resources, typst, themes }
    }

    /// Export the given presentation into PDF.
    ///
    /// This uses a separate `presenterm-export` tool.
    pub fn export_pdf(&mut self, presentation_path: &Path) -> Result<(), ExportError> {
        let metadata = self.generate_metadata(presentation_path)?;
        Self::execute_exporter(metadata).map_err(ExportError::InvokeExporter)?;
        Ok(())
    }

    /// Generate the metadata for the given presentation.
    pub fn generate_metadata(&mut self, presentation_path: &Path) -> Result<ExportMetadata, ExportError> {
        let content = fs::read_to_string(presentation_path).map_err(ExportError::ReadPresentation)?;
        let metadata = self.extract_metadata(&content, presentation_path)?;
        Ok(metadata)
    }

    /// Extract the metadata necessary to make an export.
    fn extract_metadata(&mut self, content: &str, path: &Path) -> Result<ExportMetadata, ExportError> {
        let elements = self.parser.parse(content)?;
        let path = path.canonicalize().expect("canonicalize");
        let base_path = path.parent().expect("no parent");
        let images = Self::build_image_metadata(&elements, base_path);
        let options = PresentationBuilderOptions { allow_mutations: false };
        let presentation = PresentationBuilder::new(
            CodeHighlighter::default(),
            self.default_theme,
            &mut self.resources,
            &mut self.typst,
            &self.themes,
            options,
        )
        .build(elements)?;
        Self::validate_theme_colors(&presentation)?;
        let commands = Self::build_capture_commands(presentation);
        let metadata = ExportMetadata { commands, presentation_path: path, images };
        Ok(metadata)
    }

    fn execute_exporter(metadata: ExportMetadata) -> io::Result<()> {
        let presenterm_path = env::current_exe()?;
        let mut command =
            Command::new(COMMAND).arg("--presenterm-path").arg(presenterm_path).stdin(Stdio::piped()).spawn()?;
        let mut stdin = command.stdin.take().expect("no stdin");
        let metadata = serde_json::to_vec(&metadata).expect("serialization failed");
        stdin.write_all(&metadata)?;
        stdin.flush()?;
        drop(stdin);

        let status = command.wait()?;
        if !status.success() {
            println!("PDF generation failed");
        }
        // huh?
        Ok(())
    }

    fn build_capture_commands(mut presentation: Presentation) -> Vec<CaptureCommand> {
        let mut commands = Vec::new();
        let slide_chunks: Vec<_> = presentation.iter_slides().map(|slide| slide.iter_chunks().count()).collect();
        let mut next_slide = |commands: &mut Vec<CaptureCommand>| {
            commands.push(CaptureCommand::SendKeys { keys: "l" });
            commands.push(CaptureCommand::WaitForChange);
            presentation.jump_next_slide();
        };
        for chunks in slide_chunks {
            for _ in 0..chunks - 1 {
                next_slide(&mut commands);
            }
            commands.push(CaptureCommand::Capture);
            next_slide(&mut commands);
        }
        commands
    }

    fn build_image_metadata(elements: &[MarkdownElement], base_path: &Path) -> Vec<ImageMetadata> {
        let mut positions = Vec::new();
        for element in elements {
            if let MarkdownElement::Image { path, source_position } = element {
                let full_path = base_path.join(path);
                let meta = ImageMetadata {
                    content_path: path.into(),
                    full_path,
                    line: source_position.start.line,
                    column: source_position.start.column,
                };
                positions.push(meta);
            }
        }
        positions
    }

    fn validate_theme_colors(presentation: &Presentation) -> Result<(), ExportError> {
        for slide in presentation.iter_slides() {
            for operation in slide.iter_operations() {
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

    #[error("failed to invoke presenterm-export (is it installed?): {0}")]
    InvokeExporter(io::Error),

    #[error("unsupported {0} color in theme")]
    UnsupportedColor(&'static str),
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
    content_path: PathBuf,
    full_path: PathBuf,
    line: usize,
    column: usize,
}

/// A command to whoever is capturing us indicating what to do.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum CaptureCommand {
    Capture,
    SendKeys { keys: &'static str },
    WaitForChange,
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
        let resources = Resources::new("examples");
        let typst = TypstRender::default();
        let themes = Themes::default();
        let mut exporter = Exporter::new(parser, &theme, resources, typst, themes);
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
