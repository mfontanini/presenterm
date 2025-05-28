use crate::{
    code::{
        execute::{SnippetExecutor, UnsupportedExecution},
        highlighting::{HighlightThemeSet, SnippetHighlighter},
        snippet::SnippetLanguage,
    },
    config::{KeyBindingsConfig, OptionsConfig},
    markdown::{
        elements::{
            Line, ListItem, ListItemType, MarkdownElement, Percent, PercentParseError, SourcePosition, Table, TableRow,
            Text,
        },
        parse::MarkdownParser,
        text::WeightedLine,
        text_style::{Color, Colors, TextStyle, UndefinedPaletteColorError},
    },
    presentation::{
        ChunkMutator, Modals, Presentation, PresentationMetadata, PresentationState, PresentationThemeMetadata,
        RenderOperation, SlideBuilder, SlideChunk,
    },
    render::operation::{BlockLine, ImageRenderProperties, ImageSize, MarginProperties},
    resource::Resources,
    terminal::image::{
        Image,
        printer::{ImageRegistry, ImageSpec, RegisterImageError},
    },
    theme::{
        Alignment, AuthorPositioning, ElementType, PresentationTheme, ProcessingThemeError, ThemeOptions,
        raw::{self, RawColor},
        registry::{LoadThemeError, PresentationThemeRegistry},
    },
    third_party::{ThirdPartyRender, ThirdPartyRenderError},
    ui::{
        footer::{FooterGenerator, FooterVariables, InvalidFooterTemplateError},
        modals::{IndexBuilder, KeyBindingsModalBuilder},
        separator::RenderSeparator,
    },
};
use comrak::{Arena, nodes::AlertType};
use image::DynamicImage;
use serde::Deserialize;
use snippet::{SnippetOperations, SnippetProcessor, SnippetProcessorState};
use std::{collections::HashSet, fmt::Display, iter, mem, path::PathBuf, rc::Rc, str::FromStr, sync::Arc};
use unicode_width::UnicodeWidthStr;

mod snippet;

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
        }
    }
}

/// Builds a presentation.
///
/// This type transforms [MarkdownElement]s and turns them into a presentation, which is made up of
/// render operations.
pub(crate) struct PresentationBuilder<'a> {
    slide_chunks: Vec<SlideChunk>,
    chunk_operations: Vec<RenderOperation>,
    chunk_mutators: Vec<Box<dyn ChunkMutator>>,
    slide_builders: Vec<SlideBuilder>,
    highlighter: SnippetHighlighter,
    code_executor: Arc<SnippetExecutor>,
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
    options: PresentationBuilderOptions,
}

impl<'a> PresentationBuilder<'a> {
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
        options: PresentationBuilderOptions,
    ) -> Result<Self, ProcessingThemeError> {
        let theme = PresentationTheme::new(default_raw_theme, &resources, &options.theme_options)?;
        Ok(Self {
            slide_chunks: Vec::new(),
            chunk_operations: Vec::new(),
            chunk_mutators: Vec::new(),
            slide_builders: Vec::new(),
            highlighter: SnippetHighlighter::default(),
            code_executor,
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
            options,
        })
    }

    /// Build a presentation.
    pub(crate) fn build(mut self, elements: Vec<MarkdownElement>) -> Result<Presentation, BuildError> {
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
        if last_valid { Ok(()) } else { Err(BuildError::NotInsideColumn) }
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

    fn process_front_matter(&mut self, contents: &str) -> BuildResult {
        let metadata = match self.options.strict_front_matter_parsing {
            true => serde_yaml::from_str::<StrictPresentationMetadata>(contents).map(PresentationMetadata::from),
            false => serde_yaml::from_str::<PresentationMetadata>(contents),
        };
        let mut metadata = metadata.map_err(|e| BuildError::InvalidMetadata(e.to_string()))?;
        if metadata.author.is_some() && !metadata.authors.is_empty() {
            return Err(BuildError::InvalidMetadata("cannot have both 'author' and 'authors'".into()));
        }

        if let Some(options) = metadata.options.take() {
            self.options.merge(options);
        }

        {
            let footer_context = &mut self.footer_vars;
            footer_context.title.clone_from(&metadata.title);
            footer_context.sub_title.clone_from(&metadata.sub_title);
            footer_context.location.clone_from(&metadata.location);
            footer_context.event.clone_from(&metadata.event);
            footer_context.date.clone_from(&metadata.date);
            footer_context.author.clone_from(&metadata.author);
        }

        self.set_theme(&metadata.theme)?;
        if metadata.has_frontmatter() {
            self.push_slide_prelude();
            self.push_intro_slide(metadata)?;
        }
        Ok(())
    }

    fn set_theme(&mut self, metadata: &PresentationThemeMetadata) -> BuildResult {
        if metadata.name.is_some() && metadata.path.is_some() {
            return Err(BuildError::InvalidMetadata("cannot have both theme path and theme name".into()));
        }
        let mut new_theme = None;
        // Only override the theme if we're not forced to use the default one.
        if !self.options.force_default_theme {
            if let Some(theme_name) = &metadata.name {
                let theme = self
                    .themes
                    .presentation
                    .load_by_name(theme_name)
                    .ok_or_else(|| BuildError::InvalidMetadata(format!("theme '{theme_name}' does not exist")))?;
                new_theme = Some(theme);
            }
            if let Some(theme_path) = &metadata.path {
                let mut theme = self.resources.theme(theme_path)?;
                if let Some(name) = &theme.extends {
                    let base = self
                        .themes
                        .presentation
                        .load_by_name(name)
                        .ok_or_else(|| BuildError::InvalidMetadata(format!("extended theme {name} not found")))?;
                    theme = merge_struct::merge(&theme, &base)
                        .map_err(|e| BuildError::InvalidMetadata(format!("invalid theme: {e}")))?;
                }
                new_theme = Some(theme);
            }
        }
        if let Some(overrides) = &metadata.overrides {
            if overrides.extends.is_some() {
                return Err(BuildError::InvalidMetadata("theme overrides can't use 'extends'".into()));
            }
            let base = new_theme.as_ref().unwrap_or(self.default_raw_theme);
            // This shouldn't fail as the models are already correct.
            let theme = merge_struct::merge(base, overrides)
                .map_err(|e| BuildError::InvalidMetadata(format!("invalid theme: {e}")))?;
            new_theme = Some(theme);
        }
        if let Some(theme) = new_theme {
            self.theme = PresentationTheme::new(&theme, &self.resources, &self.options.theme_options)?;
        }
        Ok(())
    }

    fn set_code_theme(&mut self) -> BuildResult {
        let theme = &self.theme.code.theme_name;
        let highlighter =
            self.themes.highlight.load_by_name(theme).ok_or_else(|| BuildError::InvalidCodeTheme(theme.clone()))?;
        self.highlighter = highlighter;
        Ok(())
    }

    fn format_presentation_title(&self, title: String) -> Result<Line, BuildError> {
        let arena = Arena::default();
        let parser = MarkdownParser::new(&arena);
        let line = parser.parse_inlines(&title).map_err(|e| BuildError::PresentationTitle(e.to_string()))?;
        let mut line = line.resolve(&self.theme.palette)?;
        line.apply_style(&self.theme.intro_slide.title.style);
        Ok(line)
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) -> BuildResult {
        let styles = &self.theme.intro_slide;

        let create_text =
            |text: Option<String>, style: TextStyle| -> Option<Text> { text.map(|text| Text::new(text, style)) };
        let title = metadata.title.map(|t| self.format_presentation_title(t)).transpose()?;

        let sub_title = create_text(metadata.sub_title, styles.subtitle.style);
        let event = create_text(metadata.event, styles.event.style);
        let location = create_text(metadata.location, styles.location.style);
        let date = create_text(metadata.date, styles.date.style);
        let authors: Vec<_> = metadata
            .author
            .into_iter()
            .chain(metadata.authors)
            .map(|author| Text::new(author, styles.author.style))
            .collect();
        if !styles.footer {
            self.slide_state.ignore_footer = true;
        }
        self.chunk_operations.push(RenderOperation::JumpToVerticalCenter);
        if let Some(title) = title {
            self.push_text(title, ElementType::PresentationTitle);
            self.push_line_break();
        }

        if let Some(sub_title) = sub_title {
            self.push_intro_slide_text(sub_title, ElementType::PresentationSubTitle);
        }
        if event.is_some() || location.is_some() || date.is_some() {
            self.push_line_breaks(2);
            if let Some(event) = event {
                self.push_intro_slide_text(event, ElementType::PresentationEvent);
            }
            if let Some(location) = location {
                self.push_intro_slide_text(location, ElementType::PresentationLocation);
            }
            if let Some(date) = date {
                self.push_intro_slide_text(date, ElementType::PresentationDate);
            }
        }
        if !authors.is_empty() {
            match self.theme.intro_slide.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.push_line_breaks(3);
                }
                AuthorPositioning::PageBottom => {
                    self.chunk_operations.push(RenderOperation::JumpToBottomRow { index: authors.len() as u16 - 1 });
                }
            };
            for author in authors {
                self.push_intro_slide_text(author, ElementType::PresentationAuthor);
            }
        }
        self.slide_state.title = Some(Line::from("[Introduction]"));
        self.terminate_slide();
        Ok(())
    }

    fn process_comment(&mut self, comment: String, source_position: SourcePosition) -> BuildResult {
        let comment = comment.trim();
        let trimmed_comment = comment.trim_start_matches(&self.options.command_prefix);
        let command = match trimmed_comment.parse::<CommentCommand>() {
            Ok(comment) => comment,
            Err(error) => {
                // If we failed to parse this, make sure we shouldn't have ignored it
                if self.should_ignore_comment(comment) {
                    return Ok(());
                }
                return Err(BuildError::CommandParse { source_position, error });
            }
        };

        if self.options.render_speaker_notes_only {
            self.process_comment_command_speaker_notes_mode(command);
            Ok(())
        } else {
            self.process_comment_command_presentation_mode(command)
        }
    }

    fn process_comment_command_presentation_mode(&mut self, command: CommentCommand) -> BuildResult {
        match command {
            CommentCommand::Pause => self.push_pause(),
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::NewLine => self.push_line_breaks(self.slide_font_size() as usize),
            CommentCommand::NewLines(count) => {
                self.push_line_breaks(count as usize * self.slide_font_size() as usize);
            }
            CommentCommand::JumpToMiddle => self.chunk_operations.push(RenderOperation::JumpToVerticalCenter),
            CommentCommand::InitColumnLayout(columns) => {
                Self::validate_column_layout(&columns)?;
                self.slide_state.layout = LayoutState::InLayout { columns_count: columns.len() };
                self.chunk_operations.push(RenderOperation::InitColumnLayout { columns });
                self.slide_state.needs_enter_column = true;
            }
            CommentCommand::ResetLayout => {
                self.slide_state.layout = LayoutState::Default;
                self.chunk_operations.extend([RenderOperation::ExitLayout, RenderOperation::RenderLineBreak]);
            }
            CommentCommand::Column(column) => {
                let (current_column, columns_count) = match self.slide_state.layout {
                    LayoutState::InColumn { column, columns_count } => (Some(column), columns_count),
                    LayoutState::InLayout { columns_count } => (None, columns_count),
                    LayoutState::Default => return Err(BuildError::NoLayout),
                };
                if current_column == Some(column) {
                    return Err(BuildError::AlreadyInColumn);
                } else if column >= columns_count {
                    return Err(BuildError::ColumnIndexTooLarge);
                }
                self.slide_state.layout = LayoutState::InColumn { column, columns_count };
                self.chunk_operations.push(RenderOperation::EnterColumn { column });
            }
            CommentCommand::IncrementalLists(value) => {
                self.slide_state.incremental_lists = Some(value);
            }
            CommentCommand::NoFooter => {
                self.slide_state.ignore_footer = true;
            }
            CommentCommand::SpeakerNote(_) => {}
            CommentCommand::FontSize(size) => {
                if size == 0 || size > 7 {
                    return Err(BuildError::InvalidFontSize);
                }
                self.slide_state.font_size = Some(size)
            }
            CommentCommand::Alignment(alignment) => {
                let alignment = match alignment {
                    CommentCommandAlignment::Left => Alignment::Left { margin: Default::default() },
                    CommentCommandAlignment::Center => {
                        Alignment::Center { minimum_margin: Default::default(), minimum_size: Default::default() }
                    }
                    CommentCommandAlignment::Right => Alignment::Right { margin: Default::default() },
                };
                self.slide_state.alignment = Some(alignment);
            }
            CommentCommand::SkipSlide => {
                self.slide_state.skip_slide = true;
            }
        };
        // Don't push line breaks for any comments.
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn process_comment_command_speaker_notes_mode(&mut self, comment_command: CommentCommand) {
        match comment_command {
            CommentCommand::SpeakerNote(note) => {
                for line in note.lines() {
                    self.push_text(line.into(), ElementType::Paragraph);
                    self.push_line_break();
                }
            }
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::SkipSlide => self.slide_state.skip_slide = true,
            _ => {}
        }
    }

    fn should_ignore_comment(&self, comment: &str) -> bool {
        if comment.contains('\n') || !comment.starts_with(&self.options.command_prefix) {
            // Ignore any multi line comment; those are assumed to be user comments
            // Ignore any line that doesn't start with the selected prefix.
            true
        } else if comment.trim().starts_with("vim:") {
            // ignore vim: commands
            true
        } else {
            // Ignore vim-like code folding tags
            let comment = comment.trim();
            comment == "{{{" || comment == "}}}"
        }
    }

    fn validate_column_layout(columns: &[u8]) -> BuildResult {
        if columns.is_empty() {
            Err(BuildError::InvalidLayout("need at least one column"))
        } else if columns.iter().any(|column| column == &0) {
            Err(BuildError::InvalidLayout("can't have zero sized columns"))
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

    fn push_slide_title(&mut self, text: Line<RawColor>) -> BuildResult {
        let mut text = text.resolve(&self.theme.palette)?;
        if self.options.implicit_slide_ends && !matches!(self.slide_state.last_element, LastElement::None) {
            self.terminate_slide();
        }

        if self.slide_state.title.is_none() {
            self.slide_state.title = Some(text.clone());
        }

        let mut style = self.theme.slide_title.clone();
        if let Some(font_size) = self.slide_state.font_size {
            style.style = style.style.size(font_size);
        }
        text.apply_style(&style.style);

        self.push_line_breaks(style.padding_top as usize);
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();

        for _ in 0..style.padding_bottom {
            self.push_line_break();
        }
        if style.separator {
            self.chunk_operations
                .push(RenderSeparator::new(Line::default(), Default::default(), style.style.size).into());
        }
        self.push_line_break();
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn push_heading(&mut self, level: u8, text: Line<RawColor>) -> BuildResult {
        let mut text = text.resolve(&self.theme.palette)?;
        let (element_type, style) = match level {
            1 => (ElementType::Heading1, &self.theme.headings.h1),
            2 => (ElementType::Heading2, &self.theme.headings.h2),
            3 => (ElementType::Heading3, &self.theme.headings.h3),
            4 => (ElementType::Heading4, &self.theme.headings.h4),
            5 => (ElementType::Heading5, &self.theme.headings.h5),
            6 => (ElementType::Heading6, &self.theme.headings.h6),
            other => panic!("unexpected heading level {other}"),
        };
        if let Some(prefix) = &style.prefix {
            if !prefix.is_empty() {
                let mut prefix = prefix.clone();
                prefix.push(' ');
                text.0.insert(0, Text::from(prefix));
            }
        }
        text.apply_style(&style.style);

        self.push_text(text, element_type);
        self.push_line_breaks(self.slide_font_size() as usize);
        Ok(())
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

    fn push_image_from_path(&mut self, path: PathBuf, title: String, source_position: SourcePosition) -> BuildResult {
        let image = self.resources.image(&path).map_err(|e| BuildError::LoadImage {
            path,
            source_position,
            error: e.to_string(),
        })?;
        self.push_image(image, title, source_position)
    }

    fn push_image(&mut self, image: Image, title: String, source_position: SourcePosition) -> BuildResult {
        let attributes = Self::parse_image_attributes(&title, &self.options.image_attribute_prefix, source_position)?;
        let size = match attributes.width {
            Some(percent) => ImageSize::WidthScaled { ratio: percent.as_ratio() },
            None => ImageSize::ShrinkIfNeeded,
        };
        let properties = ImageRenderProperties {
            size,
            background_color: self.theme.default_style.style.colors.background,
            ..Default::default()
        };
        self.chunk_operations.push(RenderOperation::RenderImage(image, properties));
        Ok(())
    }

    fn push_list(&mut self, list: Vec<ListItem>) -> BuildResult {
        let last_chunk_operation = self.slide_chunks.last().and_then(|chunk| chunk.iter_operations().last());
        // If the last chunk ended in a list, pop the newline so we get them all next to each
        // other.
        if matches!(last_chunk_operation, Some(RenderOperation::RenderLineBreak))
            && self.slide_state.last_chunk_ended_in_list
            && self.chunk_operations.is_empty()
        {
            self.slide_chunks.last_mut().unwrap().pop_last();
        }
        // If this chunk just starts (because there was a pause), pick up from the last index.
        let start_index = match self.slide_state.last_element {
            LastElement::List { last_index } if self.chunk_operations.is_empty() => last_index + 1,
            _ => 0,
        };

        let block_length =
            list.iter().map(|l| self.list_item_prefix(l).width() + l.contents.width()).max().unwrap_or_default() as u16;
        let block_length = block_length * self.slide_font_size() as u16;
        let incremental_lists = self.slide_state.incremental_lists.unwrap_or(self.options.incremental_lists);
        let iter = ListIterator::new(list, start_index);
        if incremental_lists && self.options.pause_before_incremental_lists {
            self.push_pause();
        }
        for (index, item) in iter.enumerate() {
            if index > 0 && incremental_lists {
                self.push_pause();
            }
            self.push_list_item(item.index, item.item, block_length)?;
        }
        if incremental_lists && self.options.pause_after_incremental_lists {
            self.push_pause();
        }
        Ok(())
    }

    fn push_list_item(&mut self, index: usize, item: ListItem, block_length: u16) -> BuildResult {
        let prefix = self.list_item_prefix(&item);
        let mut text = item.contents.resolve(&self.theme.palette)?;
        let font_size = self.slide_font_size();
        for piece in &mut text.0 {
            if piece.style.is_code() {
                piece.style.colors = self.theme.inline_code.style.colors;
            }
            piece.style = piece.style.size(font_size);
        }
        let alignment = self.slide_state.alignment.unwrap_or_default();
        self.chunk_operations.push(RenderOperation::RenderBlockLine(BlockLine {
            prefix: prefix.into(),
            right_padding_length: 0,
            repeat_prefix_on_wrap: false,
            text: text.into(),
            block_length,
            alignment,
            block_color: None,
        }));
        self.push_line_break();
        if item.depth == 0 {
            self.slide_state.last_element = LastElement::List { last_index: index };
        }
        Ok(())
    }

    fn list_item_prefix(&self, item: &ListItem) -> Text {
        let font_size = self.slide_font_size();
        let spaces_per_indent = match item.depth {
            0 => 3_u8.div_ceil(font_size),
            _ => {
                if font_size == 1 {
                    3
                } else {
                    2
                }
            }
        };
        let padding_length = (item.depth as usize + 1) * spaces_per_indent as usize;
        let mut prefix: String = " ".repeat(padding_length);
        match item.item_type {
            ListItemType::Unordered => {
                let delimiter = match item.depth {
                    0 => '•',
                    1 => '◦',
                    _ => '▪',
                };
                prefix.push(delimiter);
                prefix.push_str("  ");
            }
            ListItemType::OrderedParens(value) => {
                prefix.push_str(&value.to_string());
                prefix.push_str(") ");
            }
            ListItemType::OrderedPeriod(value) => {
                prefix.push_str(&value.to_string());
                prefix.push_str(". ");
            }
        };
        Text::new(prefix, TextStyle::default().size(font_size))
    }

    fn push_block_quote(&mut self, lines: Vec<Line<RawColor>>) -> BuildResult {
        let prefix = self.theme.block_quote.prefix.clone();
        let prefix_style = self.theme.block_quote.prefix_style;
        self.push_quoted_text(
            lines,
            prefix,
            self.theme.block_quote.base_style.colors,
            prefix_style,
            self.theme.block_quote.alignment,
        )
    }

    fn push_alert(
        &mut self,
        alert_type: AlertType,
        title: Option<String>,
        mut lines: Vec<Line<RawColor>>,
    ) -> BuildResult {
        let style = match alert_type {
            AlertType::Note => &self.theme.alert.styles.note,
            AlertType::Tip => &self.theme.alert.styles.tip,
            AlertType::Important => &self.theme.alert.styles.important,
            AlertType::Warning => &self.theme.alert.styles.warning,
            AlertType::Caution => &self.theme.alert.styles.caution,
        };

        let title = format!("{} {}", style.icon, title.as_deref().unwrap_or(style.title.as_ref()));
        lines.insert(0, Line::from(Text::from("")));
        lines.insert(0, Line::from(Text::new(title, style.style.into_raw())));

        let prefix = self.theme.alert.prefix.clone();
        self.push_quoted_text(
            lines,
            prefix,
            self.theme.alert.base_style.colors,
            style.style,
            self.theme.alert.alignment,
        )
    }

    fn push_quoted_text(
        &mut self,
        lines: Vec<Line<RawColor>>,
        prefix: String,
        base_colors: Colors,
        prefix_style: TextStyle,
        alignment: Alignment,
    ) -> BuildResult {
        let block_length = lines.iter().map(|line| line.width() + prefix.width()).max().unwrap_or(0) as u16;
        let font_size = self.slide_font_size();
        let prefix = Text::new(prefix, prefix_style.size(font_size));

        for line in lines {
            let mut line = line.resolve(&self.theme.palette)?;
            // Apply our colors to each chunk in this line.
            for text in &mut line.0 {
                if text.style.colors.background.is_none() && text.style.colors.foreground.is_none() {
                    text.style.colors = base_colors;
                    if text.style.is_code() {
                        text.style.colors = self.theme.inline_code.style.colors;
                    }
                }
                text.style = text.style.size(font_size);
            }
            self.chunk_operations.push(RenderOperation::RenderBlockLine(BlockLine {
                prefix: prefix.clone().into(),
                right_padding_length: 0,
                repeat_prefix_on_wrap: true,
                text: line.into(),
                block_length,
                alignment,
                block_color: base_colors.background,
            }));
            self.push_line_break();
        }
        self.set_colors(self.theme.default_style.style.colors);
        Ok(())
    }

    fn push_intro_slide_text(&mut self, text: Text, element_type: ElementType) {
        self.push_text(Line::from(text), element_type);
        self.push_line_break();
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

    fn push_code(&mut self, info: String, code: String, source_position: SourcePosition) -> BuildResult {
        let state = SnippetProcessorState {
            resources: &self.resources,
            image_registry: &self.image_registry,
            snippet_executor: self.code_executor.clone(),
            theme: &self.theme,
            third_party: self.third_party,
            highlighter: &self.highlighter,
            options: &self.options,
            font_size: self.slide_font_size(),
        };
        let processor = SnippetProcessor::new(state);
        let SnippetOperations { operations, mutators } = processor.process_code(info, code, source_position)?;
        self.chunk_operations.extend(operations);
        self.chunk_mutators.extend(mutators);
        Ok(())
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

    fn push_table(&mut self, table: Table) -> BuildResult {
        let widths: Vec<_> = (0..table.columns())
            .map(|column| table.iter_column(column).map(|text| text.width()).max().unwrap_or(0))
            .collect();
        let flattened_header = self.prepare_table_row(table.header, &widths)?;
        self.push_text(flattened_header, ElementType::Table);
        self.push_line_break();

        let mut separator = Line(Vec::new());
        for (index, width) in widths.iter().enumerate() {
            let mut contents = String::new();
            let mut margin = 1;
            if index > 0 {
                contents.push('┼');
                // Append an extra dash to have 1 column margin on both sides
                if index < widths.len() - 1 {
                    margin += 1;
                }
            }
            contents.extend(iter::repeat_n("─", *width + margin));
            separator.0.push(Text::from(contents));
        }

        self.push_text(separator, ElementType::Table);
        self.push_line_break();

        for row in table.rows {
            let flattened_row = self.prepare_table_row(row, &widths)?;
            self.push_text(flattened_row, ElementType::Table);
            self.push_line_break();
        }
        Ok(())
    }

    fn prepare_table_row(&self, row: TableRow, widths: &[usize]) -> Result<Line, BuildError> {
        let mut flattened_row = Line(Vec::new());
        for (column, text) in row.0.into_iter().enumerate() {
            let text = text.resolve(&self.theme.palette)?;
            if column > 0 {
                flattened_row.0.push(Text::from(" │ "));
            }
            let text_length = text.width();
            flattened_row.0.extend(text.0.into_iter());

            let cell_width = widths[column];
            if text_length < cell_width {
                let padding = " ".repeat(cell_width - text_length);
                flattened_row.0.push(Text::from(padding));
            }
        }
        Ok(flattened_row)
    }

    fn parse_image_attributes(
        input: &str,
        attribute_prefix: &str,
        source_position: SourcePosition,
    ) -> Result<ImageAttributes, BuildError> {
        let mut attributes = ImageAttributes::default();
        for attribute in input.split(',') {
            let Some((prefix, suffix)) = attribute.split_once(attribute_prefix) else { continue };
            if !prefix.is_empty() || (attribute_prefix.is_empty() && suffix.is_empty()) {
                continue;
            }
            Self::parse_image_attribute(suffix, &mut attributes)
                .map_err(|e| BuildError::ImageAttributeParse { source_position, error: e })?;
        }
        Ok(attributes)
    }

    fn parse_image_attribute(input: &str, attributes: &mut ImageAttributes) -> Result<(), ImageAttributeError> {
        let Some((key, value)) = input.split_once(':') else {
            return Err(ImageAttributeError::AttributeMissing);
        };
        match key {
            "width" | "w" => {
                let width = value.parse().map_err(ImageAttributeError::InvalidWidth)?;
                attributes.width = Some(width);
                Ok(())
            }
            _ => Err(ImageAttributeError::UnknownAttribute(key.to_string())),
        }
    }

    fn slide_font_size(&self) -> u8 {
        let font_size = self.slide_state.font_size.unwrap_or(1);
        if self.options.theme_options.font_size_supported { font_size.clamp(1, 7) } else { 1 }
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
    layout: LayoutState,
    title: Option<Line>,
    font_size: Option<u8>,
    alignment: Option<Alignment>,
    skip_slide: bool,
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

/// An error when building a presentation.
#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("could not load image '{path}' at {source_position}: {error}")]
    LoadImage { path: PathBuf, source_position: SourcePosition, error: String },

    #[error("failed to register image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("invalid presentation metadata: {0}")]
    InvalidMetadata(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(#[from] LoadThemeError),

    #[error("invalid code snippet at {source_position}: {error}")]
    InvalidSnippet { source_position: SourcePosition, error: String },

    #[error("invalid code highlighter theme: '{0}'")]
    InvalidCodeTheme(String),

    #[error("invalid layout: {0}")]
    InvalidLayout(&'static str),

    #[error("can't enter layout: no layout defined")]
    NoLayout,

    #[error("can't enter layout column: already in it")]
    AlreadyInColumn,

    #[error("can't enter layout column: column index too large")]
    ColumnIndexTooLarge,

    #[error("need to enter layout column explicitly using `column` command")]
    NotInsideColumn,

    #[error("invalid command at {source_position}: {error}")]
    CommandParse { source_position: SourcePosition, error: CommandParseError },

    #[error("invalid image attribute at {source_position}: {error}")]
    ImageAttributeParse { source_position: SourcePosition, error: ImageAttributeError },

    #[error("third party render failed: {0}")]
    ThirdPartyRender(#[from] ThirdPartyRenderError),

    #[error(transparent)]
    UnsupportedExecution(#[from] UnsupportedExecution),

    #[error(transparent)]
    UndefinedPaletteColor(#[from] UndefinedPaletteColorError),

    #[error("font size must be >= 1 and <= 7")]
    InvalidFontSize,

    #[error("processing theme: {0}")]
    ThemeProcessing(#[from] ProcessingThemeError),

    #[error("invalid presentation title: {0}")]
    PresentationTitle(String),

    #[error("invalid footer: {0}")]
    InvalidFooter(#[from] InvalidFooterTemplateError),
}

#[derive(Debug)]
enum ExecutionMode {
    AlongSnippet,
    ReplaceSnippet,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommentCommand {
    Pause,
    EndSlide,
    #[serde(alias = "newline")]
    NewLine,
    #[serde(alias = "newlines")]
    NewLines(u32),
    #[serde(rename = "column_layout")]
    InitColumnLayout(Vec<u8>),
    Column(usize),
    ResetLayout,
    JumpToMiddle,
    IncrementalLists(bool),
    NoFooter,
    SpeakerNote(String),
    FontSize(u8),
    Alignment(CommentCommandAlignment),
    SkipSlide,
}

impl FromStr for CommentCommand {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Deserialize)]
        struct CommandWrapper(#[serde(with = "serde_yaml::with::singleton_map")] CommentCommand);

        let wrapper = serde_yaml::from_str::<CommandWrapper>(s)?;
        Ok(wrapper.0)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommentCommandAlignment {
    Left,
    Center,
    Right,
}

#[derive(thiserror::Error, Debug)]
pub struct CommandParseError(#[from] serde_yaml::Error);

impl Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.to_string();
        // Remove the trailing "at line X, ..." that comes from serde_yaml. This otherwise claims
        // we're always in line 1 because the yaml is parsed in isolation out of the HTML comment.
        let inner = inner.split(" at line").next().unwrap();
        write!(f, "{inner}")
    }
}

struct ListIterator<I> {
    remaining: I,
    next_index: usize,
    current_depth: u8,
    saved_indexes: Vec<usize>,
}

impl<I> ListIterator<I> {
    fn new<T>(remaining: T, next_index: usize) -> Self
    where
        I: Iterator<Item = ListItem>,
        T: IntoIterator<IntoIter = I, Item = ListItem>,
    {
        Self { remaining: remaining.into_iter(), next_index, current_depth: 0, saved_indexes: Vec::new() }
    }
}

impl<I> Iterator for ListIterator<I>
where
    I: Iterator<Item = ListItem>,
{
    type Item = IndexedListItem;

    fn next(&mut self) -> Option<Self::Item> {
        let head = self.remaining.next()?;
        if head.depth != self.current_depth {
            if head.depth > self.current_depth {
                // If we're going deeper, save the next index so we can continue later on and start
                // from 0.
                self.saved_indexes.push(self.next_index);
                self.next_index = 0;
            } else {
                // if we're getting out, recover the index we had previously saved.
                for _ in head.depth..self.current_depth {
                    self.next_index = self.saved_indexes.pop().unwrap_or(0);
                }
            }
            self.current_depth = head.depth;
        }
        let index = self.next_index;
        self.next_index += 1;
        Some(IndexedListItem { index, item: head })
    }
}

#[derive(Debug)]
struct IndexedListItem {
    index: usize,
    item: ListItem,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictPresentationMetadata {
    #[serde(default)]
    title: Option<String>,

    #[serde(default)]
    sub_title: Option<String>,

    #[serde(default)]
    event: Option<String>,

    #[serde(default)]
    location: Option<String>,

    #[serde(default)]
    date: Option<String>,

    #[serde(default)]
    author: Option<String>,

    #[serde(default)]
    authors: Vec<String>,

    #[serde(default)]
    theme: PresentationThemeMetadata,

    #[serde(default)]
    options: Option<OptionsConfig>,
}

impl From<StrictPresentationMetadata> for PresentationMetadata {
    fn from(strict: StrictPresentationMetadata) -> Self {
        let StrictPresentationMetadata { title, sub_title, event, location, date, author, authors, theme, options } =
            strict;
        Self { title, sub_title, event, location, date, author, authors, theme, options }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ImageAttributeError {
    #[error("invalid width: {0}")]
    InvalidWidth(PercentParseError),

    #[error("no attribute given")]
    AttributeMissing,

    #[error("unknown attribute: '{0}'")]
    UnknownAttribute(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ImageAttributes {
    width: Option<Percent>,
}

#[cfg(test)]
mod test {
    use crate::presentation::Slide;

    use super::*;
    use rstest::rstest;

    fn build_presentation(elements: Vec<MarkdownElement>) -> Presentation {
        try_build_presentation(elements).expect("build failed")
    }

    fn build_presentation_with_options(
        elements: Vec<MarkdownElement>,
        options: PresentationBuilderOptions,
    ) -> Presentation {
        try_build_presentation_with_options(elements, options).expect("build failed")
    }

    fn try_build_presentation(elements: Vec<MarkdownElement>) -> Result<Presentation, BuildError> {
        try_build_presentation_with_options(elements, Default::default())
    }

    fn try_build_presentation_with_options(
        elements: Vec<MarkdownElement>,
        options: PresentationBuilderOptions,
    ) -> Result<Presentation, BuildError> {
        let theme = raw::PresentationTheme::default();
        let tmp_dir = std::env::temp_dir();
        let resources = Resources::new(&tmp_dir, &tmp_dir, Default::default());
        let mut third_party = ThirdPartyRender::default();
        let code_executor = Arc::new(SnippetExecutor::default());
        let themes = Themes::default();
        let bindings = KeyBindingsConfig::default();
        let builder = PresentationBuilder::new(
            &theme,
            resources,
            &mut third_party,
            code_executor,
            &themes,
            Default::default(),
            bindings,
            options,
        )?;
        builder.build(elements)
    }

    fn build_pause() -> MarkdownElement {
        MarkdownElement::Comment { comment: "pause".into(), source_position: Default::default() }
    }

    fn build_end_slide() -> MarkdownElement {
        MarkdownElement::Comment { comment: "end_slide".into(), source_position: Default::default() }
    }

    fn build_column_layout(width: u8) -> MarkdownElement {
        MarkdownElement::Comment { comment: format!("column_layout: [{width}]"), source_position: Default::default() }
    }

    fn build_column(column: u8) -> MarkdownElement {
        MarkdownElement::Comment { comment: format!("column: {column}"), source_position: Default::default() }
    }

    fn is_visible(operation: &RenderOperation) -> bool {
        use RenderOperation::*;
        match operation {
            ClearScreen
            | SetColors(_)
            | JumpToVerticalCenter
            | JumpToRow { .. }
            | JumpToColumn { .. }
            | JumpToBottomRow { .. }
            | InitColumnLayout { .. }
            | EnterColumn { .. }
            | ExitLayout
            | ApplyMargin(_)
            | PopMargin => false,
            RenderText { .. }
            | RenderLineBreak
            | RenderImage(_, _)
            | RenderBlockLine(_)
            | RenderDynamic(_)
            | RenderAsync(_) => true,
        }
    }

    fn extract_text_lines(operations: &[RenderOperation]) -> Vec<String> {
        let mut output = Vec::new();
        let mut current_line = String::new();
        for operation in operations {
            match operation {
                RenderOperation::RenderText { line, .. } => {
                    let text: String = line.iter_texts().map(|text| text.text().content.clone()).collect();
                    current_line.push_str(&text);
                }
                RenderOperation::RenderBlockLine(line) => {
                    current_line.push_str(&line.prefix.text().content);
                    current_line
                        .push_str(&line.text.iter_texts().map(|text| text.text().content.clone()).collect::<String>());
                }
                RenderOperation::RenderLineBreak if !current_line.is_empty() => {
                    output.push(mem::take(&mut current_line));
                }
                _ => (),
            };
        }
        if !current_line.is_empty() {
            output.push(current_line);
        }
        output
    }

    fn extract_slide_text_lines(slide: Slide) -> Vec<String> {
        let operations: Vec<_> = slide.into_operations().into_iter().filter(is_visible).collect();
        extract_text_lines(&operations)
    }

    #[test]
    fn empty_heading_prefix() {
        let frontmatter = r#"
theme:
  override:
    headings:
      h1:
        prefix: ""
"#;
        let elements = vec![
            MarkdownElement::FrontMatter(frontmatter.into()),
            MarkdownElement::Heading { text: "hi".into(), level: 1 },
        ];
        let mut slides = build_presentation(elements).into_slides();
        let lines = extract_slide_text_lines(slides.remove(0));
        let expected_lines = &["hi"];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn prelude_appears_once() {
        let elements = vec![
            MarkdownElement::FrontMatter("author: bob".to_string()),
            MarkdownElement::Heading { text: Line::from("hello"), level: 1 },
            build_end_slide(),
            MarkdownElement::Heading { text: Line::from("bye"), level: 1 },
        ];
        let presentation = build_presentation(elements);
        for (index, slide) in presentation.iter_slides().enumerate() {
            let clear_screen_count =
                slide.iter_visible_operations().filter(|op| matches!(op, RenderOperation::ClearScreen)).count();
            let set_colors_count =
                slide.iter_visible_operations().filter(|op| matches!(op, RenderOperation::SetColors(_))).count();
            assert_eq!(clear_screen_count, 1, "{clear_screen_count} clear screens in slide {index}");
            assert_eq!(set_colors_count, 1, "{set_colors_count} clear screens in slide {index}");
        }
    }

    #[test]
    fn slides_start_with_one_newline() {
        let elements = vec![
            MarkdownElement::FrontMatter("author: bob".to_string()),
            MarkdownElement::Heading { text: Line::from("hello"), level: 1 },
            build_end_slide(),
            MarkdownElement::Heading { text: Line::from("bye"), level: 1 },
        ];
        let presentation = build_presentation(elements);
        assert_eq!(presentation.iter_slides().count(), 3);

        // Don't process the intro slide as it's special
        let slides = presentation.into_slides().into_iter().skip(1);
        for slide in slides {
            let mut ops = slide.into_operations().into_iter().filter(is_visible);
            // We should start with a newline
            assert!(matches!(ops.next(), Some(RenderOperation::RenderLineBreak)));
            // And the second one should _not_ be a newline
            assert!(!matches!(ops.next(), Some(RenderOperation::RenderLineBreak)));
        }
    }

    #[test]
    fn table() {
        let elements = vec![MarkdownElement::Table(Table {
            header: TableRow(vec![Line::from("key"), Line::from("value"), Line::from("other")]),
            rows: vec![TableRow(vec![Line::from("potato"), Line::from("bar"), Line::from("yes")])],
        })];
        let mut slides = build_presentation(elements).into_slides();
        let lines = extract_slide_text_lines(slides.remove(0));
        let expected_lines = &["key    │ value │ other", "───────┼───────┼──────", "potato │ bar   │ yes  "];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn layout_without_init() {
        let elements = vec![build_column(0)];
        let result = try_build_presentation(elements);
        assert!(result.is_err());
    }

    #[test]
    fn already_in_column() {
        let elements = vec![
            MarkdownElement::Comment { comment: "column_layout: [1]".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "column: 0".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "column: 0".into(), source_position: Default::default() },
        ];
        let result = try_build_presentation(elements);
        assert!(result.is_err());
    }

    #[test]
    fn column_index_overflow() {
        let elements = vec![
            MarkdownElement::Comment { comment: "column_layout: [1]".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "column: 1".into(), source_position: Default::default() },
        ];
        let result = try_build_presentation(elements);
        assert!(result.is_err());
    }

    #[rstest]
    #[case::empty("column_layout: []")]
    #[case::zero("column_layout: [0]")]
    #[case::one_is_zero("column_layout: [1, 0]")]
    fn invalid_layouts(#[case] definition: &str) {
        let elements =
            vec![MarkdownElement::Comment { comment: definition.into(), source_position: Default::default() }];
        let result = try_build_presentation(elements);
        assert!(result.is_err());
    }

    #[test]
    fn operation_without_enter_column() {
        let elements = vec![
            MarkdownElement::Comment { comment: "column_layout: [1]".into(), source_position: Default::default() },
            MarkdownElement::ThematicBreak,
        ];
        let result = try_build_presentation(elements);
        assert!(result.is_err());
    }

    #[rstest]
    #[case::pause("pause", CommentCommand::Pause)]
    #[case::pause(" pause ", CommentCommand::Pause)]
    #[case::end_slide("end_slide", CommentCommand::EndSlide)]
    #[case::column_layout("column_layout: [1, 2]", CommentCommand::InitColumnLayout(vec![1, 2]))]
    #[case::column("column: 1", CommentCommand::Column(1))]
    #[case::reset_layout("reset_layout", CommentCommand::ResetLayout)]
    #[case::incremental_lists("incremental_lists: true", CommentCommand::IncrementalLists(true))]
    #[case::incremental_lists("new_lines: 2", CommentCommand::NewLines(2))]
    #[case::incremental_lists("newlines: 2", CommentCommand::NewLines(2))]
    #[case::incremental_lists("new_line", CommentCommand::NewLine)]
    #[case::incremental_lists("newline", CommentCommand::NewLine)]
    fn command_formatting(#[case] input: &str, #[case] expected: CommentCommand) {
        let parsed: CommentCommand = input.parse().expect("deserialization failed");
        assert_eq!(parsed, expected);
    }

    #[test]
    fn end_slide_inside_layout() {
        let elements = vec![build_column_layout(1), build_end_slide()];
        let presentation = build_presentation(elements);
        assert_eq!(presentation.iter_slides().count(), 2);
    }

    #[test]
    fn end_slide_inside_column() {
        let elements = vec![build_column_layout(1), build_column(0), build_end_slide()];
        let presentation = build_presentation(elements);
        assert_eq!(presentation.iter_slides().count(), 2);
    }

    #[test]
    fn pause_inside_layout() {
        let elements = vec![build_column_layout(1), build_pause(), build_column(0)];
        let presentation = build_presentation(elements);
        assert_eq!(presentation.iter_slides().count(), 1);
    }

    #[test]
    fn iterate_list() {
        let iter = ListIterator::new(
            vec![
                ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "1".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "00".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "01".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "02".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 2, contents: "001".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "2".into(), item_type: ListItemType::Unordered },
            ],
            0,
        );
        let expected_indexes = [0, 1, 0, 1, 2, 0, 2];
        let indexes: Vec<_> = iter.map(|item| item.index).collect();
        assert_eq!(indexes, expected_indexes);
    }

    #[test]
    fn iterate_list_starting_from_other() {
        let list = ListIterator::new(
            vec![
                ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "1".into(), item_type: ListItemType::Unordered },
            ],
            3,
        );
        let expected_indexes = [3, 4];
        let indexes: Vec<_> = list.into_iter().map(|item| item.index).collect();
        assert_eq!(indexes, expected_indexes);
    }

    #[test]
    fn ordered_list_with_pauses() {
        let elements = vec![
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::OrderedPeriod(1) },
                ListItem { depth: 1, contents: "one_one".into(), item_type: ListItemType::OrderedPeriod(1) },
                ListItem { depth: 1, contents: "one_two".into(), item_type: ListItemType::OrderedPeriod(2) },
            ]),
            build_pause(),
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "two".into(),
                item_type: ListItemType::OrderedPeriod(2),
            }]),
        ];
        let mut slides = build_presentation(elements).into_slides();
        let lines = extract_slide_text_lines(slides.remove(0));
        let expected_lines = &["   1. one", "      1. one_one", "      2. one_two", "   2. two"];
        assert_eq!(lines, expected_lines);
    }

    #[rstest]
    #[case::two(2, &["  •  0", "    ◦  00"])]
    #[case::three(3, &[" •  0", "    ◦  00"])]
    #[case::four(4, &[" •  0", "    ◦  00"])]
    fn list_font_size(#[case] font_size: u8, #[case] expected: &[&str]) {
        let elements = vec![
            MarkdownElement::Comment {
                comment: format!("font_size: {font_size}"),
                source_position: Default::default(),
            },
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "00".into(), item_type: ListItemType::Unordered },
            ]),
        ];
        let options = PresentationBuilderOptions {
            theme_options: ThemeOptions { font_size_supported: true },
            ..Default::default()
        };
        let mut slides = build_presentation_with_options(elements, options).into_slides();
        let lines = extract_slide_text_lines(slides.remove(0));
        assert_eq!(lines, expected);
    }

    #[rstest]
    #[case::default(Default::default(), 5)]
    #[case::no_pause_before(PresentationBuilderOptions{pause_before_incremental_lists: false, ..Default::default()}, 4)]
    #[case::no_pause_after(PresentationBuilderOptions{pause_after_incremental_lists: false, ..Default::default()}, 4)]
    #[case::no_pauses(
        PresentationBuilderOptions{
            pause_before_incremental_lists: false,
            pause_after_incremental_lists: false,
            ..Default::default()
        },
        3
    )]
    fn automatic_pauses(#[case] options: PresentationBuilderOptions, #[case] expected_chunks: usize) {
        let elements = vec![
            MarkdownElement::Comment { comment: "incremental_lists: true".into(), source_position: Default::default() },
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "two".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "three".into(), item_type: ListItemType::Unordered },
            ]),
            MarkdownElement::Paragraph(vec!["hi".into()]),
        ];
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides[0].iter_chunks().count(), expected_chunks);
    }

    #[test]
    fn automatic_pauses_no_incremental_lists() {
        let elements = vec![
            MarkdownElement::Comment {
                comment: "incremental_lists: false".into(),
                source_position: Default::default(),
            },
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "two".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "three".into(), item_type: ListItemType::Unordered },
            ]),
        ];
        let options = PresentationBuilderOptions { pause_after_incremental_lists: false, ..Default::default() };
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides[0].iter_chunks().count(), 1);
    }

    #[test]
    fn pause_new_slide() {
        let elements = vec![
            MarkdownElement::Paragraph(vec![Line::from("hi")]),
            MarkdownElement::Comment { comment: "pause".into(), source_position: Default::default() },
            MarkdownElement::Paragraph(vec![Line::from("bye")]),
        ];
        let options = PresentationBuilderOptions { pause_create_new_slide: true, ..Default::default() };
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn incremental_lists_end_of_slide() {
        let elements = vec![
            MarkdownElement::Comment { comment: "incremental_lists: true".into(), source_position: Default::default() },
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "two".into(), item_type: ListItemType::Unordered },
            ]),
        ];
        let slides = build_presentation(elements).into_slides();
        // There shouldn't be an extra one at the end
        assert_eq!(slides[0].iter_chunks().count(), 3);
    }

    #[test]
    fn skip_slide() {
        let elements = vec![
            MarkdownElement::Paragraph(vec![Line::from("hi")]),
            MarkdownElement::Comment { comment: "skip_slide".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "end_slide".into(), source_position: Default::default() },
            MarkdownElement::Paragraph(vec![Line::from("bye")]),
        ];
        let mut slides = build_presentation(elements).into_slides();
        assert_eq!(slides.len(), 1);

        let lines = extract_slide_text_lines(slides.remove(0));
        assert_eq!(lines, &["bye"]);
    }

    #[test]
    fn skip_all_slides() {
        let elements = vec![
            MarkdownElement::Paragraph(vec![Line::from("hi")]),
            MarkdownElement::Comment { comment: "skip_slide".into(), source_position: Default::default() },
        ];
        let mut slides = build_presentation(elements).into_slides();
        assert_eq!(slides.len(), 1);

        // We should still have one slide but it should be empty
        let lines = extract_slide_text_lines(slides.remove(0));
        assert_eq!(lines, Vec::<String>::new());
    }

    #[test]
    fn skip_slide_pauses() {
        let elements = vec![
            MarkdownElement::Paragraph(vec![Line::from("hi")]),
            MarkdownElement::Comment { comment: "pause".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "skip_slide".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "end_slide".into(), source_position: Default::default() },
            MarkdownElement::Paragraph(vec![Line::from("bye")]),
        ];
        let mut slides = build_presentation(elements).into_slides();
        assert_eq!(slides.len(), 1);

        let lines = extract_slide_text_lines(slides.remove(0));
        assert_eq!(lines, &["bye"]);
    }

    #[test]
    fn skip_slide_speaker_note() {
        let elements = vec![
            MarkdownElement::Paragraph(vec![Line::from("hi")]),
            MarkdownElement::Comment { comment: "skip_slide".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "end_slide".into(), source_position: Default::default() },
            MarkdownElement::Comment { comment: "speaker_note: bye".into(), source_position: Default::default() },
        ];
        let options = PresentationBuilderOptions { render_speaker_notes_only: true, ..Default::default() };
        let mut slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides.len(), 1);
        assert_eq!(extract_slide_text_lines(slides.remove(0)), &["bye"]);
    }

    #[test]
    fn pause_after_list() {
        let elements = vec![
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "one".into(),
                item_type: ListItemType::OrderedPeriod(1),
            }]),
            build_pause(),
            MarkdownElement::Heading { level: 1, text: "hi".into() },
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "two".into(),
                item_type: ListItemType::OrderedPeriod(2),
            }]),
        ];
        let slides = build_presentation(elements).into_slides();
        let first_chunk = &slides[0];
        let operations = first_chunk.iter_visible_operations().collect::<Vec<_>>();
        // This is pretty easy to break, refactor soon
        let last_operation = &operations[operations.len() - 4];
        assert!(matches!(last_operation, RenderOperation::RenderLineBreak), "last operation is {last_operation:?}");
    }

    #[test]
    fn alignment() {
        let elements = vec![
            MarkdownElement::Paragraph(vec!["hi".into()]),
            MarkdownElement::Comment { comment: "alignment: center".into(), source_position: Default::default() },
            MarkdownElement::Paragraph(vec!["hello".into()]),
            MarkdownElement::Comment { comment: "alignment: right".into(), source_position: Default::default() },
            MarkdownElement::Paragraph(vec!["hola".into()]),
        ];

        let mut slides = build_presentation(elements).into_slides();
        let operations = slides.remove(0).into_operations();
        let alignments: Vec<_> = operations
            .into_iter()
            .filter_map(|op| match op {
                RenderOperation::RenderText { alignment, .. } => Some(alignment),
                _ => None,
            })
            .collect();
        assert_eq!(
            alignments,
            &[
                Alignment::Left { margin: Default::default() },
                Alignment::Center { minimum_margin: Default::default(), minimum_size: Default::default() },
                Alignment::Right { margin: Default::default() },
            ]
        );
    }

    #[test]
    fn implicit_slide_ends() {
        let elements = vec![
            // first slide
            MarkdownElement::SetexHeading { text: "hi".into() },
            // second
            MarkdownElement::SetexHeading { text: "hi".into() },
            MarkdownElement::Heading { level: 1, text: "hi".into() },
            // explicitly ends
            MarkdownElement::Comment { comment: "end_slide".into(), source_position: Default::default() },
            // third starts
            MarkdownElement::SetexHeading { text: "hi".into() },
        ];
        let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides.len(), 3);
    }

    #[test]
    fn implicit_slide_ends_with_front_matter() {
        let elements = vec![
            MarkdownElement::FrontMatter("theme:\n name: light".into()),
            MarkdownElement::SetexHeading { text: "hi".into() },
        ];
        let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides.len(), 1);
    }

    #[rstest]
    #[case::multiline("hello\nworld")]
    #[case::many_open_braces("{{{")]
    #[case::many_close_braces("}}}")]
    #[case::vim_command("vim: hi")]
    #[case::padded_vim_command("vim: hi")]
    fn ignore_comments(#[case] comment: &str) {
        let element = MarkdownElement::Comment { comment: comment.into(), source_position: Default::default() };
        build_presentation(vec![element]);
    }

    #[rstest]
    #[case::command_with_prefix("cmd:end_slide", true)]
    #[case::non_command_with_prefix("cmd:bogus", false)]
    #[case::non_prefixed("random", true)]
    fn comment_prefix(#[case] comment: &str, #[case] should_work: bool) {
        let options = PresentationBuilderOptions { command_prefix: "cmd:".into(), ..Default::default() };

        let element = MarkdownElement::Comment { comment: comment.into(), source_position: Default::default() };
        let result = try_build_presentation_with_options(vec![element], options);
        assert_eq!(result.is_ok(), should_work, "{result:?}");
    }

    #[test]
    fn extra_fields_in_metadata() {
        let element = MarkdownElement::FrontMatter("nope: 42".into());
        let result = try_build_presentation(vec![element]);
        assert!(result.is_err());
    }

    #[test]
    fn end_slide_shorthand() {
        let options = PresentationBuilderOptions { end_slide_shorthand: true, ..Default::default() };
        let elements = vec![
            MarkdownElement::Paragraph(vec![]),
            MarkdownElement::ThematicBreak,
            MarkdownElement::Paragraph(vec!["hi".into()]),
        ];
        let presentation = build_presentation_with_options(elements, options);
        assert_eq!(presentation.iter_slides().count(), 2);

        let second = presentation.iter_slides().nth(1).unwrap();
        let before_text =
            second.iter_visible_operations().take_while(|e| !matches!(e, RenderOperation::RenderText { .. }));
        let break_count = before_text.filter(|e| matches!(e, RenderOperation::RenderLineBreak)).count();
        assert_eq!(break_count, 1);
    }

    #[test]
    fn parse_front_matter_strict() {
        let options = PresentationBuilderOptions { strict_front_matter_parsing: false, ..Default::default() };
        let elements = vec![MarkdownElement::FrontMatter("potato: yes".into())];
        let result = try_build_presentation_with_options(elements, options);
        assert!(result.is_ok());
    }

    #[rstest]
    #[case::enabled(true)]
    #[case::disabled(false)]
    fn snippet_execution(#[case] enabled: bool) {
        let element = MarkdownElement::Snippet {
            info: "rust +exec".into(),
            code: "".into(),
            source_position: Default::default(),
        };
        let options = PresentationBuilderOptions { enable_snippet_execution: enabled, ..Default::default() };
        let presentation = build_presentation_with_options(vec![element], options);
        let slide = presentation.iter_slides().next().unwrap();
        let mut found_render_block = false;
        let mut found_cant_render_block = false;
        for operation in slide.iter_visible_operations() {
            if let RenderOperation::RenderAsync(operation) = operation {
                let operation = format!("{operation:?}");
                if operation.contains("RunSnippetOperation") {
                    assert!(enabled);
                    found_render_block = true;
                } else if operation.contains("SnippetExecutionDisabledOperation") {
                    assert!(!enabled);
                    found_cant_render_block = true;
                }
            }
        }
        if found_render_block {
            assert!(enabled, "snippet execution block found but not enabled");
        } else {
            assert!(!enabled, "snippet execution enabled but not found");
        }
        if found_cant_render_block {
            assert!(!enabled, "can't execute snippet operation found but enabled");
        } else {
            assert!(enabled, "can't execute snippet operation not found");
        }
    }

    #[rstest]
    #[case::width("image:width:50%", Some(50))]
    #[case::w("image:w:50%", Some(50))]
    #[case::nothing("", None)]
    #[case::no_prefix("width", None)]
    fn image_attributes(#[case] input: &str, #[case] expectation: Option<u8>) {
        let attributes =
            PresentationBuilder::parse_image_attributes(input, "image:", Default::default()).expect("failed to parse");
        assert_eq!(attributes.width, expectation.map(Percent));
    }

    #[rstest]
    #[case::width("width:50%", Some(50))]
    #[case::empty("", None)]
    fn image_attributes_empty_prefix(#[case] input: &str, #[case] expectation: Option<u8>) {
        let attributes =
            PresentationBuilder::parse_image_attributes(input, "", Default::default()).expect("failed to parse");
        assert_eq!(attributes.width, expectation.map(Percent));
    }

    #[test]
    fn external_snippet() {
        let temp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
        let path = temp.path().file_name().expect("no file name").to_string_lossy();
        let code = format!(
            r"
path: {path}
language: rust"
        );
        let elements = vec![MarkdownElement::Snippet {
            info: "file +line_numbers +exec".into(),
            code,
            source_position: Default::default(),
        }];
        build_presentation(elements);
    }

    #[test]
    fn footnote() {
        let elements = vec![MarkdownElement::Footnote(Line::from("hi")), MarkdownElement::Footnote(Line::from("bye"))];
        let mut slides = build_presentation(elements).into_slides();
        let text = extract_slide_text_lines(slides.remove(0));
        assert_eq!(text, &["hi", "bye"]);
    }
}
