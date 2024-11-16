use super::{
    code::{CodeBlockParser, CodeLine, ExternalFile, Highlight, HighlightGroup, Snippet, SnippetLanguage},
    execution::{DisplaySeparator, RunAcquireTerminalSnippet, SnippetExecutionDisabledOperation},
    modals::KeyBindingsModalBuilder,
};
use crate::{
    custom::{KeyBindingsConfig, OptionsConfig},
    execute::SnippetExecutor,
    markdown::{
        elements::{
            ListItem, ListItemType, MarkdownElement, Percent, PercentParseError, SourcePosition, Table, TableRow, Text,
            TextBlock,
        },
        text::WeightedTextBlock,
    },
    media::{image::Image, printer::RegisterImageError, register::ImageRegistry},
    presentation::{
        AsRenderOperations, BlockLine, ChunkMutator, ImageProperties, ImageSize, MarginProperties, Modals,
        Presentation, PresentationMetadata, PresentationState, PresentationThemeMetadata, RenderAsync, RenderOperation,
        Slide, SlideBuilder, SlideChunk,
    },
    processing::{
        code::{CodePreparer, HighlightContext, HighlightMutator, HighlightedLine},
        execution::RunSnippetOperation,
        footer::{FooterContext, FooterGenerator},
        modals::IndexBuilder,
        separator::RenderSeparator,
    },
    render::{
        highlighting::{CodeHighlighter, HighlightThemeSet},
        properties::WindowSize,
    },
    resource::{LoadImageError, Resources},
    style::{Color, Colors, TextStyle},
    theme::{
        Alignment, AuthorPositioning, CodeBlockStyle, ElementType, LoadThemeError, Margin, PresentationTheme,
        PresentationThemeSet,
    },
    third_party::{ThirdPartyRender, ThirdPartyRenderError, ThirdPartyRenderRequest},
};
use image::DynamicImage;
use serde::Deserialize;
use std::{borrow::Cow, cell::RefCell, fmt::Display, iter, mem, path::PathBuf, rc::Rc, str::FromStr};
use unicode_width::UnicodeWidthStr;

// TODO: move to a theme config.
static DEFAULT_BOTTOM_SLIDE_MARGIN: u16 = 3;
pub(crate) static DEFAULT_IMAGE_Z_INDEX: i32 = -2;

#[derive(Default)]
pub struct Themes {
    pub presentation: PresentationThemeSet,
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
    slides: Vec<Slide>,
    highlighter: CodeHighlighter,
    code_executor: Rc<SnippetExecutor>,
    theme: Cow<'a, PresentationTheme>,
    resources: &'a mut Resources,
    third_party: &'a mut ThirdPartyRender,
    slide_state: SlideState,
    presentation_state: PresentationState,
    footer_context: Rc<RefCell<FooterContext>>,
    themes: &'a Themes,
    index_builder: IndexBuilder,
    image_registry: ImageRegistry,
    bindings_config: KeyBindingsConfig,
    options: PresentationBuilderOptions,
}

impl<'a> PresentationBuilder<'a> {
    /// Construct a new builder.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        default_theme: &'a PresentationTheme,
        resources: &'a mut Resources,
        third_party: &'a mut ThirdPartyRender,
        code_executor: Rc<SnippetExecutor>,
        themes: &'a Themes,
        image_registry: ImageRegistry,
        bindings_config: KeyBindingsConfig,
        options: PresentationBuilderOptions,
    ) -> Self {
        Self {
            slide_chunks: Vec::new(),
            chunk_operations: Vec::new(),
            chunk_mutators: Vec::new(),
            slides: Vec::new(),
            highlighter: CodeHighlighter::default(),
            code_executor,
            theme: Cow::Borrowed(default_theme),
            resources,
            third_party,
            slide_state: Default::default(),
            presentation_state: Default::default(),
            footer_context: Default::default(),
            themes,
            index_builder: Default::default(),
            image_registry,
            bindings_config,
            options,
        }
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
        self.footer_context.borrow_mut().total_slides = self.slides.len();

        let mut bindings_modal_builder = KeyBindingsModalBuilder::default();
        if self.options.print_modal_background {
            let background = self.build_modal_background()?;
            self.index_builder.set_background(background.clone());
            bindings_modal_builder.set_background(background);
        };

        let slide_index = self.index_builder.build(&self.theme, self.presentation_state.clone());
        let bindings = bindings_modal_builder.build(&self.theme, &self.bindings_config);
        let modals = Modals { slide_index, bindings };
        let presentation = Presentation::new(self.slides, modals, self.presentation_state);
        Ok(presentation)
    }

    fn build_modal_background(&self) -> Result<Image, RegisterImageError> {
        let color = self
            .theme
            .modals
            .colors
            .background
            .as_ref()
            .or(self.theme.default_style.colors.background.as_ref())
            .and_then(Color::as_rgb);
        // If we don't have an rgb color (or we don't have a color at all), we default to a dark
        // background.
        let rgba = match color {
            Some((r, g, b)) => [r, g, b, 255],
            None => [0, 0, 0, 255],
        };
        let mut image = DynamicImage::new_rgba8(1, 1);
        image.as_mut_rgba8().unwrap().get_pixel_mut(0, 0).0 = rgba;
        let image = self.image_registry.register_image(image)?;
        Ok(image)
    }

    fn validate_last_operation(&mut self) -> Result<(), BuildError> {
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

    fn push_slide_prelude(&mut self) {
        let colors = self.theme.default_style.colors;
        self.chunk_operations.extend([
            RenderOperation::SetColors(colors),
            RenderOperation::ClearScreen,
            RenderOperation::ApplyMargin(MarginProperties {
                horizontal_margin: self.theme.default_style.margin.clone().unwrap_or_default(),
                bottom_slide_margin: DEFAULT_BOTTOM_SLIDE_MARGIN,
            }),
        ]);
        self.push_line_break();
    }

    fn process_element_for_presentation_mode(&mut self, element: MarkdownElement) -> Result<(), BuildError> {
        let should_clear_last = !matches!(element, MarkdownElement::List(_) | MarkdownElement::Comment { .. });
        match element {
            // This one is processed before everything else as it affects how the rest of the
            // elements is rendered.
            MarkdownElement::FrontMatter(_) => self.slide_state.ignore_element_line_break = true,
            MarkdownElement::SetexHeading { text } => self.push_slide_title(text),
            MarkdownElement::Heading { level, text } => self.push_heading(level, text),
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements)?,
            MarkdownElement::List(elements) => self.push_list(elements),
            MarkdownElement::Snippet { info, code, source_position } => self.push_code(info, code, source_position)?,
            MarkdownElement::Table(table) => self.push_table(table),
            MarkdownElement::ThematicBreak => self.process_thematic_break(),
            MarkdownElement::Comment { comment, source_position } => self.process_comment(comment, source_position)?,
            MarkdownElement::BlockQuote(lines) => self.push_block_quote(lines),
            MarkdownElement::Image { path, title, source_position } => {
                self.push_image_from_path(path, title, source_position)?
            }
        };
        if should_clear_last {
            self.slide_state.last_element = LastElement::Other;
        }
        Ok(())
    }

    fn process_element_for_speaker_notes_mode(&mut self, element: MarkdownElement) -> Result<(), BuildError> {
        match element {
            MarkdownElement::Comment { comment, source_position } => self.process_comment(comment, source_position)?,
            MarkdownElement::SetexHeading { text } => self.push_slide_title(text),
            _ => {}
        }
        // Allows us to start the next speaker slide when a title is pushed and implicit_slide_ends is enabled.
        self.slide_state.last_element = LastElement::Other;
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn process_front_matter(&mut self, contents: &str) -> Result<(), BuildError> {
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
            let mut footer_context = self.footer_context.borrow_mut();
            footer_context.title = metadata.title.clone().unwrap_or_default();
            footer_context.sub_title = metadata.sub_title.clone().unwrap_or_default();
            footer_context.location = metadata.location.clone().unwrap_or_default();
            footer_context.event = metadata.event.clone().unwrap_or_default();
            footer_context.date = metadata.date.clone().unwrap_or_default();
            footer_context.author = metadata.author.clone().unwrap_or_default();
        }

        self.set_theme(&metadata.theme)?;
        if metadata.has_frontmatter() {
            self.push_slide_prelude();
            self.push_intro_slide(metadata);
        }
        Ok(())
    }

    fn set_theme(&mut self, metadata: &PresentationThemeMetadata) -> Result<(), BuildError> {
        if metadata.name.is_some() && metadata.path.is_some() {
            return Err(BuildError::InvalidMetadata("cannot have both theme path and theme name".into()));
        }
        // Only override the theme if we're not forced to use the defaul theme if we're not forced
        // to use the default one.
        if !self.options.force_default_theme {
            if let Some(theme_name) = &metadata.name {
                let theme = self
                    .themes
                    .presentation
                    .load_by_name(theme_name)
                    .ok_or_else(|| BuildError::InvalidMetadata(format!("theme '{theme_name}' does not exist")))?;
                self.theme = Cow::Owned(theme);
            }
            if let Some(theme_path) = &metadata.path {
                let theme = self.resources.theme(theme_path)?;
                self.theme = Cow::Owned(theme);
            }
        }
        if let Some(overrides) = &metadata.overrides {
            if overrides.extends.is_some() {
                return Err(BuildError::InvalidMetadata("theme overrides can't use 'extends'".into()));
            }
            // This shouldn't fail as the models are already correct.
            let theme = merge_struct::merge(self.theme.as_ref(), overrides)
                .map_err(|e| BuildError::InvalidMetadata(format!("invalid theme: {e}")))?;
            self.theme = Cow::Owned(theme);
        }
        Ok(())
    }

    fn set_code_theme(&mut self) -> Result<(), BuildError> {
        if let Some(theme) = &self.theme.code.theme_name {
            let highlighter =
                self.themes.highlight.load_by_name(theme).ok_or_else(|| BuildError::InvalidCodeTheme(theme.clone()))?;
            self.highlighter = highlighter;
        }
        Ok(())
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) {
        let styles = self.theme.intro_slide.clone();
        let create_text =
            |text: Option<String>, style: TextStyle| -> Option<Text> { text.map(|text| Text::new(text, style)) };
        let title = create_text(metadata.title, TextStyle::default().bold().colors(styles.title.colors));
        let sub_title = create_text(metadata.sub_title, TextStyle::default().colors(styles.subtitle.colors));
        let event = create_text(metadata.event, TextStyle::default().colors(styles.event.colors));
        let location = create_text(metadata.location, TextStyle::default().colors(styles.location.colors));
        let date = create_text(metadata.date, TextStyle::default().colors(styles.date.colors));
        let authors: Vec<_> = metadata
            .author
            .into_iter()
            .chain(metadata.authors)
            .map(|author| Text::new(author, TextStyle::default().colors(styles.author.colors)))
            .collect();
        if styles.footer == Some(false) {
            self.slide_state.ignore_footer = true;
        }
        self.chunk_operations.push(RenderOperation::JumpToVerticalCenter);
        if let Some(title) = title {
            self.push_line(title, ElementType::PresentationTitle);
        }
        if let Some(sub_title) = sub_title {
            self.push_line(sub_title, ElementType::PresentationSubTitle);
        }
        if event.is_some() || location.is_some() || date.is_some() {
            self.push_line_break();
            self.push_line_break();
            if let Some(event) = event {
                self.push_line(event, ElementType::PresentationEvent);
            }
            if let Some(location) = location {
                self.push_line(location, ElementType::PresentationLocation);
            }
            if let Some(date) = date {
                self.push_line(date, ElementType::PresentationDate);
            }
        }
        if !authors.is_empty() {
            match self.theme.intro_slide.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.push_line_break();
                    self.push_line_break();
                    self.push_line_break();
                }
                AuthorPositioning::PageBottom => {
                    self.chunk_operations.push(RenderOperation::JumpToBottomRow { index: authors.len() as u16 - 1 });
                }
            };
            for author in authors {
                self.push_line(author, ElementType::PresentationAuthor);
            }
        }
        self.slide_state.title = Some(TextBlock::from("[Introduction]"));
        self.terminate_slide();
    }

    fn process_comment(&mut self, comment: String, source_position: SourcePosition) -> Result<(), BuildError> {
        let comment = comment.trim();
        if self.should_ignore_comment(comment) {
            return Ok(());
        }
        let comment = comment.trim_start_matches(&self.options.command_prefix);
        let comment = match comment.parse::<CommentCommand>() {
            Ok(comment) => comment,
            Err(error) => return Err(BuildError::CommandParse { line: source_position.start.line + 1, error }),
        };

        if self.options.render_speaker_notes_only {
            match comment {
                CommentCommand::SpeakerNote(note) => {
                    self.push_text(note.into(), ElementType::Paragraph);
                    self.push_line_break();
                }
                CommentCommand::EndSlide => self.terminate_slide(),
                _ => {}
            }
            return Ok(());
        }

        match comment {
            CommentCommand::Pause => self.process_pause(),
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::NewLine => self.push_line_break(),
            CommentCommand::NewLines(count) => {
                for _ in 0..count {
                    self.push_line_break();
                }
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
        };
        // Don't push line breaks for any comments.
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn should_ignore_comment(&self, comment: &str) -> bool {
        if comment.contains('\n') || !comment.starts_with(&self.options.command_prefix) {
            // Ignore any multi line comment; those are assumed to be user comments
            // Ignore any line that doesn't start with the selected prefix.
            true
        } else {
            // Ignore vim-like code folding tags
            let comment = comment.trim();
            comment == "{{{" || comment == "}}}"
        }
    }

    fn validate_column_layout(columns: &[u8]) -> Result<(), BuildError> {
        if columns.is_empty() {
            Err(BuildError::InvalidLayout("need at least one column"))
        } else if columns.iter().any(|column| column == &0) {
            Err(BuildError::InvalidLayout("can't have zero sized columns"))
        } else {
            Ok(())
        }
    }

    fn process_pause(&mut self) {
        self.slide_state.last_chunk_ended_in_list = matches!(self.slide_state.last_element, LastElement::List { .. });

        let chunk_operations = mem::take(&mut self.chunk_operations);
        let mutators = mem::take(&mut self.chunk_mutators);
        self.slide_chunks.push(SlideChunk::new(chunk_operations, mutators));
    }

    fn push_slide_title(&mut self, mut text: TextBlock) {
        if self.options.implicit_slide_ends && !matches!(self.slide_state.last_element, LastElement::None) {
            self.terminate_slide();
        }

        if self.slide_state.title.is_none() {
            self.slide_state.title = Some(text.clone());
        }

        let style = self.theme.slide_title.clone();
        let mut text_style = TextStyle::default().colors(style.colors);
        if style.bold.unwrap_or_default() {
            text_style = text_style.bold();
        }
        if style.italics.unwrap_or_default() {
            text_style = text_style.italics();
        }
        if style.underlined.unwrap_or_default() {
            text_style = text_style.underlined();
        }
        text.apply_style(&text_style);

        for _ in 0..style.padding_top.unwrap_or(0) {
            self.push_line_break();
        }
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();

        for _ in 0..style.padding_bottom.unwrap_or(0) {
            self.push_line_break();
        }
        if style.separator {
            self.chunk_operations.push(RenderSeparator::default().into());
        }
        self.push_line_break();
        self.slide_state.ignore_element_line_break = true;
    }

    fn push_heading(&mut self, level: u8, mut text: TextBlock) {
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
            let mut prefix = prefix.clone();
            prefix.push(' ');
            text.0.insert(0, Text::from(prefix));
        }
        let text_style = TextStyle::default().bold().colors(style.colors);
        text.apply_style(&text_style);

        self.push_text(text, element_type);
        self.push_line_break();
    }

    fn push_paragraph(&mut self, lines: Vec<TextBlock>) -> Result<(), BuildError> {
        for text in lines {
            self.push_text(text, ElementType::Paragraph);
            self.push_line_break();
        }
        Ok(())
    }

    fn process_thematic_break(&mut self) {
        if self.options.end_slide_shorthand {
            self.terminate_slide();
            self.slide_state.ignore_element_line_break = true;
        } else {
            self.chunk_operations.extend([RenderSeparator::default().into(), RenderOperation::RenderLineBreak]);
        }
    }

    fn push_image_from_path(
        &mut self,
        path: PathBuf,
        title: String,
        source_position: SourcePosition,
    ) -> Result<(), BuildError> {
        let image = self.resources.image(&path).map_err(|e| BuildError::LoadImage(path, e))?;
        self.push_image(image, title, source_position)
    }

    fn push_image(&mut self, image: Image, title: String, source_position: SourcePosition) -> Result<(), BuildError> {
        let attributes = Self::parse_image_attributes(&title, &self.options.image_attribute_prefix, source_position)?;
        let size = match attributes.width {
            Some(percent) => ImageSize::WidthScaled { ratio: percent.as_ratio() },
            None => ImageSize::ShrinkIfNeeded,
        };
        let properties = ImageProperties {
            z_index: DEFAULT_IMAGE_Z_INDEX,
            size,
            restore_cursor: false,
            background_color: self.theme.default_style.colors.background,
        };
        self.chunk_operations.extend([
            RenderOperation::RenderImage(image, properties),
            RenderOperation::SetColors(self.theme.default_style.colors),
        ]);
        Ok(())
    }

    fn push_list(&mut self, list: Vec<ListItem>) {
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

        let incremental_lists = self.slide_state.incremental_lists.unwrap_or(self.options.incremental_lists);
        let iter = ListIterator::new(list, start_index);
        for (index, item) in iter.enumerate() {
            if index > 0 && incremental_lists {
                self.process_pause();
            }
            self.push_list_item(item.index, item.item);
        }
    }

    fn push_list_item(&mut self, index: usize, item: ListItem) {
        let padding_length = (item.depth as usize + 1) * 3;
        let mut prefix: String = " ".repeat(padding_length);
        match item.item_type {
            ListItemType::Unordered => {
                let delimiter = match item.depth {
                    0 => '•',
                    1 => '◦',
                    _ => '▪',
                };
                prefix.push(delimiter);
            }
            ListItemType::OrderedParens => {
                prefix.push_str(&(index + 1).to_string());
                prefix.push_str(") ");
            }
            ListItemType::OrderedPeriod => {
                prefix.push_str(&(index + 1).to_string());
                prefix.push_str(". ");
            }
        };

        let prefix_length = prefix.len() as u16;
        self.push_text(prefix.into(), ElementType::List);

        let text = item.contents;
        self.push_aligned_text(text, Alignment::Left { margin: Margin::Fixed(prefix_length) });
        self.push_line_break();
        if item.depth == 0 {
            self.slide_state.last_element = LastElement::List { last_index: index };
        }
    }

    fn push_block_quote(&mut self, lines: Vec<TextBlock>) {
        let prefix = self.theme.block_quote.prefix.clone().unwrap_or_default();
        let block_length = lines.iter().map(|line| line.width() + prefix.width()).max().unwrap_or(0) as u16;
        let prefix_color = self.theme.block_quote.colors.prefix.or(self.theme.block_quote.colors.base.foreground);
        let prefix = Text::new(
            prefix,
            TextStyle::default()
                .colors(Colors { foreground: prefix_color, background: self.theme.block_quote.colors.base.background }),
        );
        let alignment = self.theme.alignment(&ElementType::BlockQuote).clone();

        for mut line in lines {
            // Apply our colors to each chunk in this line.
            for text in &mut line.0 {
                text.style.colors = self.theme.block_quote.colors.base;
                if text.style.is_code() {
                    text.style.colors = self.theme.inline_code.colors;
                }
            }
            self.chunk_operations.push(RenderOperation::RenderBlockLine(BlockLine {
                prefix: prefix.clone().into(),
                right_padding_length: 0,
                repeat_prefix_on_wrap: true,
                text: line.into(),
                block_length,
                alignment: alignment.clone(),
                block_color: self.theme.block_quote.colors.base.background,
            }));
            self.push_line_break();
        }
        self.chunk_operations.push(RenderOperation::SetColors(self.theme.default_style.colors));
    }

    fn push_line(&mut self, text: Text, element_type: ElementType) {
        self.push_text(TextBlock::from(text), element_type);
        self.push_line_break();
    }

    fn push_text(&mut self, text: TextBlock, element_type: ElementType) {
        let alignment = self.theme.alignment(&element_type);
        self.push_aligned_text(text, alignment);
    }

    fn push_aligned_text(&mut self, mut block: TextBlock, alignment: Alignment) {
        for chunk in &mut block.0 {
            if chunk.style.is_code() {
                chunk.style.colors = self.theme.inline_code.colors;
            }
        }
        if !block.0.is_empty() {
            self.chunk_operations.push(RenderOperation::RenderText {
                line: WeightedTextBlock::from(block),
                alignment: alignment.clone(),
            });
        }
    }

    fn push_line_break(&mut self) {
        self.chunk_operations.push(RenderOperation::RenderLineBreak);
    }

    fn push_differ(&mut self, text: String) {
        self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(Differ(text))));
    }

    fn push_code(&mut self, info: String, code: String, source_position: SourcePosition) -> Result<(), BuildError> {
        let mut snippet = CodeBlockParser::parse(info, code)
            .map_err(|e| BuildError::InvalidCode { line: source_position.start.line + 1, error: e.to_string() })?;
        if matches!(snippet.language, SnippetLanguage::File) {
            snippet = self.load_external_snippet(snippet, source_position.clone())?;
        }
        self.push_differ(snippet.contents.clone());

        if snippet.attributes.auto_render {
            return self.push_rendered_code(snippet, source_position);
        } else if snippet.attributes.execute_replace && self.options.enable_snippet_execution_replace {
            return self.push_code_execution(snippet, 0, ExecutionMode::ReplaceSnippet);
        }
        let lines =
            CodePreparer::new(&self.theme, self.code_executor.hidden_line_prefix(&snippet.language)).prepare(&snippet);
        let block_length = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        let (lines, context) = self.highlight_lines(&snippet, lines, block_length);
        for line in lines {
            self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(line)));
        }
        self.chunk_operations.push(RenderOperation::SetColors(self.theme.default_style.colors));
        if self.options.allow_mutations && context.borrow().groups.len() > 1 {
            self.chunk_mutators.push(Box::new(HighlightMutator::new(context)));
        }

        if snippet.attributes.execute_replace && !self.options.enable_snippet_execution_replace {
            let operation = SnippetExecutionDisabledOperation::new(
                self.theme.execution_output.status.failure,
                self.theme.code.alignment.clone().unwrap_or_default(),
            );
            operation.start_render();
            self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(operation)))
        }
        if snippet.attributes.execute {
            if self.options.enable_snippet_execution {
                self.push_code_execution(snippet, block_length, ExecutionMode::AlongSnippet)?;
            } else {
                let operation = SnippetExecutionDisabledOperation::new(
                    self.theme.execution_output.status.failure,
                    self.theme.code.alignment.clone().unwrap_or_default(),
                );
                self.chunk_operations.push(RenderOperation::RenderAsync(Rc::new(operation)))
            }
        }
        Ok(())
    }

    fn load_external_snippet(
        &mut self,
        mut code: Snippet,
        source_position: SourcePosition,
    ) -> Result<Snippet, BuildError> {
        // TODO clean up this repeated thing
        let line = source_position.start.line + 1;
        let file: ExternalFile =
            serde_yaml::from_str(&code.contents).map_err(|e| BuildError::InvalidCode { line, error: e.to_string() })?;
        let path = file.path;
        let path_display = path.display();
        let contents = self
            .resources
            .external_snippet(&path)
            .map_err(|e| BuildError::InvalidCode { line, error: format!("failed to load {path_display}: {e}") })?;
        code.language = file.language;
        code.contents = contents;
        Ok(code)
    }

    fn push_rendered_code(&mut self, code: Snippet, source_position: SourcePosition) -> Result<(), BuildError> {
        let Snippet { contents, language, attributes } = code;
        let error_holder = self.presentation_state.async_error_holder();
        let request = match language {
            SnippetLanguage::Typst => ThirdPartyRenderRequest::Typst(contents, self.theme.typst.clone()),
            SnippetLanguage::Latex => ThirdPartyRenderRequest::Latex(contents, self.theme.typst.clone()),
            SnippetLanguage::Mermaid => ThirdPartyRenderRequest::Mermaid(contents, self.theme.mermaid.clone()),
            _ => {
                return Err(BuildError::InvalidCode {
                    line: source_position.start.line + 1,
                    error: format!("language {language:?} doesn't support rendering"),
                })?;
            }
        };
        let operation =
            self.third_party.render(request, &self.theme, error_holder, self.slides.len() + 1, attributes.width)?;
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn highlight_lines(
        &self,
        code: &Snippet,
        lines: Vec<CodeLine>,
        block_length: usize,
    ) -> (Vec<HighlightedLine>, Rc<RefCell<HighlightContext>>) {
        let mut code_highlighter = self.highlighter.language_highlighter(&code.language);
        let style = self.code_style(code);
        let dim_style = {
            let mut highlighter = self.highlighter.language_highlighter(&SnippetLanguage::Rust);
            highlighter.style_line("//", &style).0.first().expect("no styles").style
        };
        let groups = match self.options.allow_mutations {
            true => code.attributes.highlight_groups.clone(),
            false => vec![HighlightGroup::new(vec![Highlight::All])],
        };
        let context = Rc::new(RefCell::new(HighlightContext {
            groups,
            current: 0,
            block_length,
            alignment: style.alignment.clone().unwrap_or_default(),
        }));

        let mut output = Vec::new();
        for line in lines.into_iter() {
            let prefix = line.dim_prefix(&dim_style);
            let highlighted = line.highlight(&mut code_highlighter, &style);
            let not_highlighted = line.dim(&dim_style);
            let line_number = line.line_number;
            let context = context.clone();
            output.push(HighlightedLine {
                prefix,
                right_padding_length: line.right_padding_length,
                highlighted,
                not_highlighted,
                line_number,
                context,
                block_color: dim_style.colors.background,
            });
        }
        (output, context)
    }

    fn code_style(&self, snippet: &Snippet) -> CodeBlockStyle {
        let mut style = self.theme.code.clone();
        if snippet.attributes.no_background {
            style.alignment = match style.alignment {
                Some(Alignment::Center { .. }) => {
                    Some(Alignment::Center { minimum_size: 0, minimum_margin: Margin::default() })
                }
                Some(Alignment::Left { .. }) => Some(Alignment::Left { margin: Margin::default() }),
                Some(Alignment::Right { .. }) => Some(Alignment::Right { margin: Margin::default() }),
                None => None,
            };
            style.background = Some(false);
        }
        style
    }

    fn push_code_execution(
        &mut self,
        code: Snippet,
        block_length: usize,
        mode: ExecutionMode,
    ) -> Result<(), BuildError> {
        if !self.code_executor.is_execution_supported(&code.language) {
            return Err(BuildError::UnsupportedExecution(code.language));
        }
        if code.attributes.acquire_terminal {
            let block_length = block_length as u16;
            let block_length = match self.theme.code.alignment.clone().unwrap_or_default() {
                Alignment::Left { .. } | Alignment::Right { .. } => block_length,
                Alignment::Center { minimum_size, .. } => block_length.max(minimum_size),
            };
            let operation = RunAcquireTerminalSnippet::new(
                code,
                self.code_executor.clone(),
                self.theme.execution_output.status.clone(),
                block_length,
            );
            let operation = RenderOperation::RenderAsync(Rc::new(operation));
            self.chunk_operations.push(operation);
            return Ok(());
        }
        let separator = match mode {
            ExecutionMode::AlongSnippet => DisplaySeparator::On,
            ExecutionMode::ReplaceSnippet => DisplaySeparator::Off,
        };
        let alignment = self.code_style(&code).alignment.unwrap_or_default();
        let default_colors = self.theme.default_style.colors;
        let mut execution_output_style = self.theme.execution_output.clone();
        if code.attributes.no_background {
            execution_output_style.colors.background = None;
        }
        let operation = RunSnippetOperation::new(
            code,
            self.code_executor.clone(),
            default_colors,
            execution_output_style,
            block_length as u16,
            separator,
            alignment,
        );
        if matches!(mode, ExecutionMode::ReplaceSnippet) {
            operation.start_render();
        }
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn terminate_slide(&mut self) {
        let footer = self.generate_footer();

        let operations = mem::take(&mut self.chunk_operations);
        let mutators = mem::take(&mut self.chunk_mutators);
        self.slide_chunks.push(SlideChunk::new(operations, mutators));

        let chunks = mem::take(&mut self.slide_chunks);
        let slide = SlideBuilder::default().chunks(chunks).footer(footer).build();
        self.index_builder.add_title(self.slide_state.title.take().unwrap_or_else(|| Text::from("<no title>").into()));
        self.slides.push(slide);

        self.push_slide_prelude();
        self.slide_state = Default::default();
        self.slide_state.last_element = LastElement::None;
    }

    fn generate_footer(&mut self) -> Vec<RenderOperation> {
        if self.slide_state.ignore_footer {
            return Vec::new();
        }
        let generator = FooterGenerator {
            style: self.theme.footer.clone().unwrap_or_default(),
            current_slide: self.slides.len(),
            context: self.footer_context.clone(),
        };
        vec![
            // Exit any layout we're in so this gets rendered on a default screen size.
            RenderOperation::ExitLayout,
            // Pop the slide margin so we're at the terminal rect.
            RenderOperation::PopMargin,
            RenderOperation::RenderDynamic(Rc::new(generator)),
        ]
    }

    fn push_table(&mut self, table: Table) {
        let widths: Vec<_> = (0..table.columns())
            .map(|column| table.iter_column(column).map(|text| text.width()).max().unwrap_or(0))
            .collect();
        let flattened_header = Self::prepare_table_row(table.header, &widths);
        self.push_text(flattened_header, ElementType::Table);
        self.push_line_break();

        let mut separator = TextBlock(Vec::new());
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
            contents.extend(iter::repeat("─").take(*width + margin));
            separator.0.push(Text::from(contents));
        }

        self.push_text(separator, ElementType::Table);
        self.push_line_break();

        for row in table.rows {
            let flattened_row = Self::prepare_table_row(row, &widths);
            self.push_text(flattened_row, ElementType::Table);
            self.push_line_break();
        }
    }

    fn prepare_table_row(row: TableRow, widths: &[usize]) -> TextBlock {
        let mut flattened_row = TextBlock(Vec::new());
        for (column, text) in row.0.into_iter().enumerate() {
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
        flattened_row
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
                .map_err(|e| BuildError::ImageAttributeParse { line: source_position.start.line + 1, error: e })?;
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
    title: Option<TextBlock>,
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
    #[error("failed to load image '{0}': {1}")]
    LoadImage(PathBuf, LoadImageError),

    #[error("failed to register image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("invalid presentation metadata: {0}")]
    InvalidMetadata(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(#[from] LoadThemeError),

    #[error("invalid code at line {line}: {error}")]
    InvalidCode { line: usize, error: String },

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

    #[error("error parsing command at line {line}: {error}")]
    CommandParse { line: usize, error: CommandParseError },

    #[error("error parsing image attribute at line {line}: {error}")]
    ImageAttributeParse { line: usize, error: ImageAttributeError },

    #[error("third party render failed: {0}")]
    ThirdPartyRender(#[from] ThirdPartyRenderError),

    #[error("language {0:?} does not support execution")]
    UnsupportedExecution(SnippetLanguage),
}

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

#[derive(Debug)]
struct Differ(String);

impl AsRenderOperations for Differ {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        Vec::new()
    }

    fn diffable_content(&self) -> Option<&str> {
        Some(&self.0)
    }
}

#[cfg(test)]
mod test {
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
        let theme = PresentationTheme::default();
        let mut resources = Resources::new("/tmp", Default::default());
        let mut third_party = ThirdPartyRender::default();
        let code_executor = Rc::new(SnippetExecutor::default());
        let themes = Themes::default();
        let bindings = KeyBindingsConfig::default();
        let builder = PresentationBuilder::new(
            &theme,
            &mut resources,
            &mut third_party,
            code_executor,
            &themes,
            Default::default(),
            bindings,
            options,
        );
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
            | JumpToBottomRow { .. }
            | InitColumnLayout { .. }
            | EnterColumn { .. }
            | ExitLayout { .. }
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
                    let texts: Vec<_> = line.iter_texts().map(|text| text.text().content.clone()).collect();
                    current_line.push_str(&texts.join(""));
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
    fn prelude_appears_once() {
        let elements = vec![
            MarkdownElement::FrontMatter("author: bob".to_string()),
            MarkdownElement::Heading { text: TextBlock::from("hello"), level: 1 },
            build_end_slide(),
            MarkdownElement::Heading { text: TextBlock::from("bye"), level: 1 },
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
            MarkdownElement::Heading { text: TextBlock::from("hello"), level: 1 },
            build_end_slide(),
            MarkdownElement::Heading { text: TextBlock::from("bye"), level: 1 },
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
            header: TableRow(vec![TextBlock::from("key"), TextBlock::from("value"), TextBlock::from("other")]),
            rows: vec![TableRow(vec![TextBlock::from("potato"), TextBlock::from("bar"), TextBlock::from("yes")])],
        })];
        let slides = build_presentation(elements).into_slides();
        let lines = extract_slide_text_lines(slides.into_iter().next().unwrap());
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
            vec![ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered }, ListItem {
                depth: 0,
                contents: "1".into(),
                item_type: ListItemType::Unordered,
            }],
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
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::OrderedPeriod },
                ListItem { depth: 1, contents: "one_one".into(), item_type: ListItemType::OrderedPeriod },
                ListItem { depth: 1, contents: "one_two".into(), item_type: ListItemType::OrderedPeriod },
            ]),
            build_pause(),
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "two".into(),
                item_type: ListItemType::OrderedPeriod,
            }]),
        ];
        let slides = build_presentation(elements).into_slides();
        let lines = extract_slide_text_lines(slides.into_iter().next().unwrap());
        let expected_lines = &["   1. one", "      1. one_one", "      2. one_two", "   2. two"];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn automatic_pauses() {
        let elements = vec![
            MarkdownElement::Comment { comment: "incremental_lists: true".into(), source_position: Default::default() },
            MarkdownElement::List(vec![
                ListItem { depth: 0, contents: "one".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "two".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "three".into(), item_type: ListItemType::Unordered },
            ]),
        ];
        let slides = build_presentation(elements).into_slides();
        assert_eq!(slides[0].iter_chunks().count(), 3);
    }

    #[test]
    fn pause_after_list() {
        let elements = vec![
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "one".into(),
                item_type: ListItemType::OrderedPeriod,
            }]),
            build_pause(),
            MarkdownElement::Heading { level: 1, text: "hi".into() },
            MarkdownElement::List(vec![ListItem {
                depth: 0,
                contents: "two".into(),
                item_type: ListItemType::OrderedPeriod,
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
        let elements =
            vec![MarkdownElement::FrontMatter("theme:\n name: light".into()), MarkdownElement::SetexHeading {
                text: "hi".into(),
            }];
        let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
        let slides = build_presentation_with_options(elements, options).into_slides();
        assert_eq!(slides.len(), 1);
    }

    #[rstest]
    #[case::multiline("hello\nworld")]
    #[case::many_open_braces("{{{")]
    #[case::many_close_braces("}}}")]
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
}
