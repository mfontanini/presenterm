use crate::{
    builder::{BuildError, PresentationBuilder},
    markdown::{elements::MarkdownElement, parse::ParseError},
    presentation::Presentation,
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
    default_highlighter: CodeHighlighter,
    resources: Resources,
}

impl<'a> Exporter<'a> {
    /// Construct a new exporter.
    pub fn new(
        parser: MarkdownParser<'a>,
        default_theme: &'a PresentationTheme,
        default_highlighter: CodeHighlighter,
        resources: Resources,
    ) -> Self {
        Self { parser, default_theme, default_highlighter, resources }
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
        let base_path = path.parent().expect("no parent").canonicalize().expect("canonicalize");
        let images = Self::build_image_metadata(&elements, &base_path);
        let presentation =
            PresentationBuilder::new(self.default_highlighter.clone(), self.default_theme, &mut self.resources)
                .build(elements)?;
        let commands = Self::build_capture_commands(presentation);
        let presentation_path = path.canonicalize().map_err(ExportError::ReadPresentation)?;
        let metadata = ExportMetadata { commands, presentation_path, images };
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
    use comrak::Arena;

    fn extract_metadata(content: &str, path: &str) -> ExportMetadata {
        let arena = Arena::new();
        let parser = MarkdownParser::new(&arena);
        let theme = Default::default();
        let highlighter = CodeHighlighter::new("base16-ocean.dark").unwrap();
        let resources = Resources::new("examples");
        let mut exporter = Exporter::new(parser, &theme, highlighter, resources);
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
