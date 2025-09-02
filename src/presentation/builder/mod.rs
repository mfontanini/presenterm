use crate::{
    code::{
        execute::SnippetExecutor,
        highlighting::{HighlightThemeSet, SnippetHighlighter},
        snippet::SnippetLanguage,
    },
    config::{KeyBindingsConfig, OptionsConfig},
    markdown::{
        elements::{Line, MarkdownElement, SourcePosition, Text},
        parse::MarkdownParser,
        text::WeightedLine,
        text_style::{Color, Colors},
    },
    presentation::{
        ChunkMutator, Modals, Presentation, PresentationState, RenderOperation, SlideBuilder, SlideChunk,
        builder::{
            error::{BuildError, ErrorContextBuilder, FileSourcePosition, InvalidPresentation},
            sources::MarkdownSources,
        },
    },
    render::operation::MarginProperties,
    resource::{ResourceBasePath, Resources},
    terminal::image::{
        Image,
        printer::{ImageRegistry, ImageSpec, RegisterImageError},
    },
    theme::{
        Alignment, ElementType, PresentationTheme, ProcessingThemeError, ThemeOptions,
        raw::{self, RawColor},
        registry::PresentationThemeRegistry,
    },
    third_party::ThirdPartyRender,
    ui::{
        execution::output::SnippetHandle,
        footer::{FooterGenerator, FooterVariables},
        modals::{IndexBuilder, KeyBindingsModalBuilder},
        separator::RenderSeparator,
    },
};
use image::DynamicImage;
use std::{
    collections::{HashMap, HashSet},
    fs, io, iter, mem,
    path::Path,
    rc::Rc,
    sync::Arc,
};

pub(crate) mod error;

mod comment;
mod frontmatter;
mod heading;
mod images;
mod list;
mod quote;
mod snippet;
mod sources;
mod table;

#[cfg(test)]
mod tests;

pub(crate) type BuildResult = Result<(), BuildError>;

#[derive(Default)]
pub struct Themes {
    pub presentation: PresentationThemeRegistry,
    pub highlight: HighlightThemeSet,
}

#[derive(Clone, Debug)]
pub struct PresentationBuilderOptions {
    pub allow_mutations: bool,
    pub implicit_slide_ends: bool,
    pub command_prefix: String,
    pub image_attribute_prefix: String,
    pub incremental_lists: bool,
    pub force_default_theme: bool,
    pub end_slide_shorthand: bool,
    pub print_modal_background: bool,
    pub strict_front_matter_parsing: bool,
    pub enable_snippet_execution: bool,
    pub enable_snippet_execution_replace: bool,
    pub render_speaker_notes_only: bool,
    pub auto_render_languages: Vec<SnippetLanguage>,
    pub theme_options: ThemeOptions,
    pub pause_before_incremental_lists: bool,
    pub pause_after_incremental_lists: bool,
    pub pause_create_new_slide: bool,
    pub list_item_newlines: u8,
    pub validate_snippets: bool,
    pub layout_grid: bool,
}

impl PresentationBuilderOptions {
    fn merge(&mut self, options: OptionsConfig) {
        self.implicit_slide_ends = options.implicit_slide_ends.unwrap_or(self.implicit_slide_ends);
        self.incremental_lists = options.incremental_lists.unwrap_or(self.incremental_lists);
        self.end_slide_shorthand = options.end_slide_shorthand.unwrap_or(self.end_slide_shorthand);
        self.strict_front_matter_parsing =
            options.strict_front_matter_parsing.unwrap_or(self.strict_front_matter_parsing);
        if let Some(prefix) = options.command_prefix {
            self.command_prefix = prefix;
        }
        if let Some(prefix) = options.image_attributes_prefix {
            self.image_attribute_prefix = prefix;
        }
        if !options.auto_render_languages.is_empty() {
            self.auto_render_languages = options.auto_render_languages;
        }
        if let Some(count) = options.list_item_newlines {
            self.list_item_newlines = count.into();
        }
    }
}

impl Default for PresentationBuilderOptions {
    fn default() -> Self {
        Self {
            allow_mutations: true,
            implicit_slide_ends: false,
            command_prefix: String::default(),
            image_attribute_prefix: "image:".to_string(),
            incremental_lists: false,
            force_default_theme: false,
            end_slide_shorthand: false,
            print_modal_background: false,
            strict_front_matter_parsing: true,
            enable_snippet_execution: false,
            enable_snippet_execution_replace: false,
            render_speaker_notes_only: false,
            auto_render_languages: Default::default(),
            theme_options: ThemeOptions { font_size_supported: false },
            pause_before_incremental_lists: true,
            pause_after_incremental_lists: true,
            pause_create_new_slide: false,
            list_item_newlines: 1,
            validate_snippets: false,
            layout_grid: false,
        }
    }
}

/// Builds a presentation.
///
/// This type transforms [MarkdownElement]s and turns them into a presentation, which is made up of
/// render operations.
pub(crate) struct PresentationBuilder<'a, 'b> {
    slide_chunks: Vec<SlideChunk>,
    chunk_operations: Vec<RenderOperation>,
    chunk_mutators: Vec<Box<dyn ChunkMutator>>,
    slide_builders: Vec<SlideBuilder>,
    highlighter: SnippetHighlighter,
    snippet_executor: Arc<SnippetExecutor>,
    theme: PresentationTheme,
    default_raw_theme: &'a raw::PresentationTheme,
    resources: Resources,
    third_party: &'a mut ThirdPartyRender,
    slide_state: SlideState,
    presentation_state: PresentationState,
    footer_vars: FooterVariables,
    themes: &'a Themes,
    index_builder: IndexBuilder,
    image_registry: ImageRegistry,
    bindings_config: KeyBindingsConfig,
    slides_without_footer: HashSet<usize>,
    markdown_parser: &'a MarkdownParser<'b>,
    executable_snippets: HashMap<String, SnippetHandle>,
    sources: MarkdownSources,
    options: PresentationBuilderOptions,
}

impl<'a, 'b> PresentationBuilder<'a, 'b> {
    /// Construct a new builder.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        default_raw_theme: &'a raw::PresentationTheme,
        resources: Resources,
        third_party: &'a mut ThirdPartyRender,
        code_executor: Arc<SnippetExecutor>,
        themes: &'a Themes,
        image_registry: ImageRegistry,
        bindings_config: KeyBindingsConfig,
        markdown_parser: &'a MarkdownParser<'b>,
        options: PresentationBuilderOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let theme = PresentationTheme::new(default_raw_theme, &resources, &options.theme_options)?;
        Ok(Self {
            slide_chunks: Vec::new(),
            chunk_operations: Vec::new(),
            chunk_mutators: Vec::new(),
            slide_builders: Vec::new(),
            highlighter: SnippetHighlighter::default(),
            snippet_executor: code_executor,
            theme,
            default_raw_theme,
            resources,
            third_party,
            slide_state: Default::default(),
            presentation_state: Default::default(),
            footer_vars: Default::default(),
            themes,
            index_builder: Default::default(),
            image_registry,
            bindings_config,
            slides_without_footer: HashSet::new(),
            markdown_parser,
            sources: Default::default(),
            executable_snippets: Default::default(),
            options,
        })
    }

    /// Build a presentation from a markdown input.
    pub(crate) fn build(self, path: &Path) -> Result<Presentation, BuildError> {
        self.build_with_reader(path, FilesystemPresentationReader)
    }

    /// Build a presentation from already parsed elements.
    pub(crate) fn build_from_parsed(mut self, elements: Vec<MarkdownElement>) -> Result<Presentation, BuildError> {
        let mut skip_first = false;
        if let Some(MarkdownElement::FrontMatter(contents)) = elements.first() {
            self.process_front_matter(contents)?;
            skip_first = true;
        }
        let mut elements = elements.into_iter();
        if skip_first {
            elements.next();
        }

        self.set_code_theme()?;

        if self.chunk_operations.is_empty() {
            self.push_slide_prelude();
        }
        for element in elements {
            self.slide_state.ignore_element_line_break = false;
            if self.options.render_speaker_notes_only {
                self.process_element_for_speaker_notes_mode(element)?;
            } else {
                self.process_element_for_presentation_mode(element)?;
            }
            self.validate_last_operation()?;
            if !self.slide_state.ignore_element_line_break {
                self.push_line_break();
            }
        }
        if !self.chunk_operations.is_empty() || !self.slide_chunks.is_empty() {
            self.terminate_slide();
        }

        // Always have at least one empty slide
        if self.slide_builders.is_empty() {
            self.terminate_slide();
        }

        let mut bindings_modal_builder = KeyBindingsModalBuilder::default();
        if self.options.print_modal_background {
            let background = self.build_modal_background()?;
            self.index_builder.set_background(background.clone());
            bindings_modal_builder.set_background(background);
        };

        let mut slides = Vec::new();
        let builders = mem::take(&mut self.slide_builders);
        self.footer_vars.total_slides = builders.len();
        for (index, mut builder) in builders.into_iter().enumerate() {
            self.footer_vars.current_slide = index + 1;
            if !self.slides_without_footer.contains(&index) {
                builder = builder.footer(self.generate_footer()?);
            }
            slides.push(builder.build());
        }

        let bindings = bindings_modal_builder.build(&self.theme, &self.bindings_config);
        let slide_index = self.index_builder.build(&self.theme, self.presentation_state.clone());
        let modals = Modals { slide_index, bindings };
        let presentation = Presentation::new(slides, modals, self.presentation_state);
        Ok(presentation)
    }

    fn build_with_reader<F: PresentationReader>(self, path: &Path, reader: F) -> Result<Presentation, BuildError> {
        let _guard = self.sources.enter(path).map_err(BuildError::EnterRoot)?;
        let contents = reader.read(path).map_err(|e| BuildError::ReadPresentation(path.into(), e))?;
        let elements = self.markdown_parser.parse(&contents).map_err(|error| {
            let context =
                ErrorContextBuilder::new(&contents, &error.kind.to_string()).position(error.sourcepos).build();
            BuildError::Parse { path: path.into(), error, context }
        })?;
        self.build_from_parsed(elements)
    }

    fn build_modal_background(&self) -> Result<Image, RegisterImageError> {
        let color = self.theme.modals.style.colors.background.as_ref().and_then(Color::as_rgb);
        // If we don't have an rgb color (or we don't have a color at all), we default to a dark
        // background.
        let rgba = match color {
            Some((r, g, b)) => [r, g, b, 255],
            None => [0, 0, 0, 255],
        };
        let mut image = DynamicImage::new_rgba8(1, 1);
        image.as_mut_rgba8().unwrap().get_pixel_mut(0, 0).0 = rgba;
        let image = self.image_registry.register(ImageSpec::Generated(image))?;
        Ok(image)
    }

    fn validate_last_operation(&mut self) -> BuildResult {
        if !self.slide_state.needs_enter_column {
            return Ok(());
        }
        let Some(last) = self.chunk_operations.last() else {
            return Ok(());
        };
        if matches!(last, RenderOperation::InitColumnLayout { .. }) {
            return Ok(());
        }
        self.slide_state.needs_enter_column = false;
        let last_valid = matches!(last, RenderOperation::EnterColumn { .. } | RenderOperation::ExitLayout);
        if last_valid {
            Ok(())
        } else {
            let position = self.slide_state.last_layout_comment.as_ref().expect("no last position");
            let context = fs::read_to_string(&position.file)
                .ok()
                .map(|s| {
                    ErrorContextBuilder::new(&s, "layout was created here").position(position.source_position).build()
                })
                .unwrap_or_default();
            Err(BuildError::NotInsideColumn(context))
        }
    }

    fn set_colors(&mut self, colors: Colors) {
        self.chunk_operations.push(RenderOperation::SetColors(colors));
    }

    fn push_slide_prelude(&mut self) {
        let style = self.theme.default_style.style;
        self.set_colors(style.colors);

        let footer_height = self.theme.footer.height();
        self.chunk_operations.extend([
            RenderOperation::ClearScreen,
            RenderOperation::ApplyMargin(MarginProperties {
                horizontal: self.theme.default_style.margin,
                top: 0,
                bottom: footer_height,
            }),
        ]);
        self.push_line_break();
    }

    fn process_element_for_presentation_mode(&mut self, element: MarkdownElement) -> BuildResult {
        let should_clear_last = !matches!(element, MarkdownElement::List(_) | MarkdownElement::Comment { .. });
        match element {
            // This one is processed before everything else as it affects how the rest of the
            // elements is rendered.
            MarkdownElement::FrontMatter(_) => self.slide_state.ignore_element_line_break = true,
            MarkdownElement::SetexHeading { text } => self.push_slide_title(text)?,
            MarkdownElement::Heading { level, text } => self.push_heading(level, text)?,
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements)?,
            MarkdownElement::List(elements) => self.push_list(elements)?,
            MarkdownElement::Snippet { info, code, source_position } => self.push_code(info, code, source_position)?,
            MarkdownElement::Table(table) => self.push_table(table)?,
            MarkdownElement::ThematicBreak => self.process_thematic_break(),
            MarkdownElement::Comment { comment, source_position } => self.process_comment(comment, source_position)?,
            MarkdownElement::BlockQuote(lines) => self.push_block_quote(lines)?,
            MarkdownElement::Image { path, title, source_position } => {
                self.push_image_from_path(path, title, source_position)?
            }
            MarkdownElement::Alert { alert_type, title, lines } => self.push_alert(alert_type, title, lines)?,
            MarkdownElement::Footnote(line) => {
                let line = line.resolve(&self.theme.palette)?;
                self.push_text(line, ElementType::Paragraph);
            }
        };
        if should_clear_last {
            self.slide_state.last_element = LastElement::Other;
        }
        Ok(())
    }

    fn process_element_for_speaker_notes_mode(&mut self, element: MarkdownElement) -> BuildResult {
        match element {
            MarkdownElement::Comment { comment, source_position } => self.process_comment(comment, source_position)?,
            MarkdownElement::SetexHeading { text } => self.push_slide_title(text)?,
            MarkdownElement::ThematicBreak => {
                if self.options.end_slide_shorthand {
                    self.terminate_slide();
                    self.slide_state.ignore_element_line_break = true;
                }
            }
            _ => {}
        }
        // Allows us to start the next speaker slide when a title is pushed and implicit_slide_ends is enabled.
        self.slide_state.last_element = LastElement::Other;
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn set_code_theme(&mut self) -> BuildResult {
        let theme = &self.theme.code.theme_name;
        let highlighter =
            self.themes.highlight.load_by_name(theme).ok_or_else(|| BuildError::InvalidCodeTheme(theme.clone()))?;
        self.highlighter = highlighter;
        Ok(())
    }

    fn invalid_presentation<E>(&self, source_position: SourcePosition, error: E) -> BuildError
    where
        E: Into<InvalidPresentation>,
    {
        let error = error.into();
        let source_position = self.sources.resolve_source_position(source_position);
        let context = fs::read_to_string(&source_position.file)
            .ok()
            .map(|s| ErrorContextBuilder::new(&s, &error.to_string()).position(source_position.source_position).build())
            .unwrap_or_default();

        let FileSourcePosition { source_position, file } = source_position;
        BuildError::InvalidPresentation { source_position, path: file, context }
    }

    fn resource_base_path(&self) -> ResourceBasePath {
        ResourceBasePath::Custom(self.sources.current_base_path())
    }

    fn validate_column_layout(&self, columns: &[u8], source_position: SourcePosition) -> BuildResult {
        if columns.is_empty() {
            Err(self
                .invalid_presentation(source_position, InvalidPresentation::InvalidLayout("need at least one column")))
        } else if columns.iter().any(|column| column == &0) {
            Err(self.invalid_presentation(
                source_position,
                InvalidPresentation::InvalidLayout("can't have zero sized columns"),
            ))
        } else {
            Ok(())
        }
    }

    fn push_pause(&mut self) {
        if self.options.pause_create_new_slide {
            let operations = self.chunk_operations.clone();
            self.terminate_slide();
            self.chunk_operations = operations;
            return;
        }
        self.slide_state.last_chunk_ended_in_list = matches!(self.slide_state.last_element, LastElement::List { .. });

        let chunk_operations = mem::take(&mut self.chunk_operations);
        let mutators = mem::take(&mut self.chunk_mutators);
        self.slide_chunks.push(SlideChunk::new(chunk_operations, mutators));
    }

    fn push_paragraph(&mut self, lines: Vec<Line<RawColor>>) -> BuildResult {
        for line in lines {
            let line = line.resolve(&self.theme.palette)?;
            self.push_text(line, ElementType::Paragraph);
            self.push_line_breaks(self.slide_font_size() as usize);
        }
        Ok(())
    }

    fn process_thematic_break(&mut self) {
        if self.options.end_slide_shorthand {
            self.terminate_slide();
            self.slide_state.ignore_element_line_break = true;
        } else {
            self.chunk_operations.extend([
                RenderSeparator::new(Line::default(), Default::default(), self.slide_font_size()).into(),
                RenderOperation::RenderLineBreak,
            ]);
        }
    }

    fn push_text(&mut self, line: Line, element_type: ElementType) {
        let alignment = self.slide_state.alignment.unwrap_or_else(|| self.theme.alignment(&element_type));
        self.push_aligned_text(line, alignment);
    }

    fn push_aligned_text(&mut self, mut block: Line, alignment: Alignment) {
        let default_font_size = self.slide_font_size();
        for chunk in &mut block.0 {
            if chunk.style.is_code() {
                chunk.style.colors = self.theme.inline_code.style.colors;
            }
            if default_font_size > 1 {
                chunk.style = chunk.style.size(default_font_size);
            }
        }
        if !block.0.is_empty() {
            self.chunk_operations.push(RenderOperation::RenderText { line: WeightedLine::from(block), alignment });
        }
    }

    fn push_line_break(&mut self) {
        self.push_line_breaks(1)
    }

    fn push_line_breaks(&mut self, count: usize) {
        self.chunk_operations.extend(iter::repeat_n(RenderOperation::RenderLineBreak, count));
    }

    fn terminate_slide(&mut self) {
        let operations = mem::take(&mut self.chunk_operations);
        let mutators = mem::take(&mut self.chunk_mutators);
        // Don't allow a last empty pause in slide since it adds nothing
        if self.slide_chunks.is_empty() || !Self::is_chunk_empty(&operations) {
            self.slide_chunks.push(SlideChunk::new(operations, mutators));
        }
        let chunks = mem::take(&mut self.slide_chunks);

        if !self.slide_state.skip_slide {
            let builder = SlideBuilder::default().chunks(chunks);
            self.index_builder
                .add_title(self.slide_state.title.take().unwrap_or_else(|| Text::from("<no title>").into()));

            if self.slide_state.ignore_footer {
                self.slides_without_footer.insert(self.slide_builders.len());
            }
            self.slide_builders.push(builder);
        }

        self.push_slide_prelude();
        self.slide_state = Default::default();
    }

    fn is_chunk_empty(operations: &[RenderOperation]) -> bool {
        if operations.is_empty() {
            return true;
        }
        for operation in operations {
            if !matches!(operation, RenderOperation::RenderLineBreak) {
                return false;
            }
        }
        true
    }

    fn generate_footer(&self) -> Result<Vec<RenderOperation>, BuildError> {
        let generator = FooterGenerator::new(self.theme.footer.clone(), &self.footer_vars, &self.theme.palette)?;
        Ok(vec![
            // Exit any layout we're in so this gets rendered on a default screen size.
            RenderOperation::ExitLayout,
            // Pop the slide margin so we're at the terminal rect.
            RenderOperation::PopMargin,
            RenderOperation::RenderDynamic(Rc::new(generator)),
        ])
    }

    fn slide_font_size(&self) -> u8 {
        let font_size = self.slide_state.font_size.unwrap_or(1);
        if self.options.theme_options.font_size_supported { font_size.clamp(1, 7) } else { 1 }
    }
}

trait PresentationReader {
    fn read(&self, path: &Path) -> io::Result<String>;
}

struct FilesystemPresentationReader;

impl PresentationReader for FilesystemPresentationReader {
    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }
}

#[derive(Debug, Default)]
struct SlideState {
    ignore_element_line_break: bool,
    ignore_footer: bool,
    needs_enter_column: bool,
    last_chunk_ended_in_list: bool,
    last_element: LastElement,
    incremental_lists: Option<bool>,
    list_item_newlines: Option<u8>,
    layout: LayoutState,
    title: Option<Line>,
    font_size: Option<u8>,
    alignment: Option<Alignment>,
    skip_slide: bool,
    last_layout_comment: Option<FileSourcePosition>,
}

#[derive(Debug, Default)]
enum LayoutState {
    #[default]
    Default,
    InLayout {
        columns_count: usize,
    },
    InColumn {
        column: usize,
        columns_count: usize,
    },
}

#[derive(Debug, Default)]
enum LastElement {
    #[default]
    None,
    List {
        last_index: usize,
    },
    Other,
}

#[cfg(test)]
pub(crate) mod utils {
    use super::*;
    use crate::{
        render::{engine::RenderEngine, properties::WindowSize},
        terminal::virt::VirtualTerminal,
    };
    use std::{path::PathBuf, thread::sleep, time::Duration};

    struct MemoryPresentationReader {
        contents: String,
    }

    impl PresentationReader for MemoryPresentationReader {
        fn read(&self, _path: &Path) -> io::Result<String> {
            Ok(self.contents.clone())
        }
    }

    pub(crate) enum Input {
        Markdown(String),
        Parsed(Vec<MarkdownElement>),
    }

    impl From<&'_ str> for Input {
        fn from(value: &'_ str) -> Self {
            Self::Markdown(value.to_string())
        }
    }

    impl From<String> for Input {
        fn from(value: String) -> Self {
            Self::Markdown(value)
        }
    }

    impl From<Vec<MarkdownElement>> for Input {
        fn from(value: Vec<MarkdownElement>) -> Self {
            Self::Parsed(value)
        }
    }

    pub(crate) struct Test {
        input: Input,
        options: PresentationBuilderOptions,
        resources_path: PathBuf,
        theme: raw::PresentationTheme,
    }

    impl Test {
        pub(crate) fn new<T: Into<Input>>(input: T) -> Self {
            let options = PresentationBuilderOptions {
                enable_snippet_execution: true,
                enable_snippet_execution_replace: true,
                theme_options: ThemeOptions { font_size_supported: true },
                ..Default::default()
            };
            Self { input: input.into(), options, resources_path: std::env::temp_dir(), theme: Default::default() }
        }

        pub(crate) fn options(mut self, options: PresentationBuilderOptions) -> Self {
            self.options = options;
            self
        }

        pub(crate) fn resources_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
            self.resources_path = path.into();
            self
        }

        pub(crate) fn theme(mut self, theme: raw::PresentationTheme) -> Self {
            self.theme = theme;
            self
        }

        pub(crate) fn disable_exec_replace(mut self) -> Self {
            self.options.enable_snippet_execution_replace = false;
            self
        }

        pub(crate) fn disable_exec(mut self) -> Self {
            self.options.enable_snippet_execution = false;
            self
        }

        pub(crate) fn with_builder<T, F>(&self, callback: F) -> T
        where
            F: for<'a, 'b> Fn(PresentationBuilder<'a, 'b>) -> T,
        {
            let theme = &self.theme;
            let resources = Resources::new(&self.resources_path, &self.resources_path, Default::default());
            let mut third_party = ThirdPartyRender::default();
            let code_executor = Arc::new(SnippetExecutor::default());
            let themes = Themes::default();
            let bindings = KeyBindingsConfig::default();
            let arena = Default::default();
            let parser = MarkdownParser::new(&arena);
            let builder = PresentationBuilder::new(
                theme,
                resources,
                &mut third_party,
                code_executor,
                &themes,
                Default::default(),
                bindings,
                &parser,
                self.options.clone(),
            )
            .expect("failed to create builder");
            callback(builder)
        }

        pub(crate) fn render(self) -> PresentationRender {
            let presentation = self.build();
            PresentationRender::new(presentation)
        }

        pub(crate) fn build(self) -> Presentation {
            self.try_build().expect("build failed")
        }

        pub(crate) fn expect_invalid(self) -> BuildError {
            self.try_build().expect_err("build succeeded")
        }

        pub(crate) fn try_build(self) -> Result<Presentation, BuildError> {
            self.with_builder(|builder| match &self.input {
                Input::Markdown(input) => {
                    let reader = MemoryPresentationReader { contents: input.clone() };
                    let path = self.resources_path.join("presentation.md");
                    builder.build_with_reader(&path, reader)
                }
                Input::Parsed(elements) => builder.build_from_parsed(elements.clone()),
            })
        }
    }

    pub(crate) struct PresentationRender {
        presentation: Presentation,
        columns: Option<u16>,
        rows: Option<u16>,
        run_async_renders: bool,
        background_maps: Vec<(Color, char)>,
        advances: Option<usize>,
    }

    impl PresentationRender {
        fn new(presentation: Presentation) -> Self {
            Self {
                presentation,
                columns: None,
                rows: None,
                run_async_renders: true,
                background_maps: Default::default(),
                advances: None,
            }
        }

        pub(crate) fn rows(mut self, rows: u16) -> Self {
            self.rows = Some(rows);
            self
        }

        pub(crate) fn columns(mut self, columns: u16) -> Self {
            self.columns = Some(columns);
            self
        }

        pub(crate) fn advances(mut self, number: usize) -> Self {
            self.advances = Some(number);
            self
        }

        pub(crate) fn run_async_renders(mut self, value: bool) -> Self {
            self.run_async_renders = value;
            self
        }

        pub(crate) fn map_background(mut self, color: Color, c: char) -> Self {
            self.background_maps.push((color, c));
            self
        }

        pub(crate) fn into_lines(self) -> Vec<String> {
            self.into_parts().0
        }

        pub(crate) fn into_parts(self) -> (Vec<String>, Vec<String>) {
            let Self { mut presentation, columns, rows, run_async_renders, background_maps, advances } = self;
            let columns = columns.expect("no columns");
            let rows = rows.expect("no rows");
            let dimensions = WindowSize { rows, columns, width: 0, height: 0 };
            let only_visible = advances.is_some();
            if let Some(advances) = advances {
                for _ in 0..advances {
                    presentation.jump_next();
                }
            }

            let slide = presentation.current_slide_mut();
            if run_async_renders {
                for operation in slide.iter_operations_mut() {
                    if let RenderOperation::RenderAsync(operation) = operation {
                        let mut pollable = operation.pollable();
                        while !pollable.poll().is_completed() {
                            sleep(Duration::from_millis(1));
                        }
                    }
                }
            }

            let mut term = VirtualTerminal::new(dimensions, Default::default());
            let engine = RenderEngine::new(&mut term, dimensions, Default::default());
            if only_visible {
                engine.render(slide.iter_visible_operations()).expect("failed to render");
            } else {
                engine.render(slide.iter_operations()).expect("failed to render");
            }
            let mut lines = Vec::new();
            let mut styles = Vec::new();
            for row in term.into_contents().rows {
                let mut line = String::new();
                let mut style = String::new();
                for character in &row {
                    let style_char = background_maps
                        .iter()
                        .filter_map(|(b, c)| (character.style.colors.background == Some(*b)).then_some(c))
                        .next()
                        .unwrap_or(&' ');
                    line.push(character.character);
                    style.push(*style_char);
                }
                lines.push(line);
                styles.push(style);
            }
            (lines, styles)
        }
    }
}
