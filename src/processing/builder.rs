use crate::{
    custom::{KeyBindingsConfig, OptionsConfig},
    markdown::{
        elements::{
            Code, CodeLanguage, Highlight, HighlightGroup, ListItem, ListItemType, MarkdownElement, ParagraphElement,
            SourcePosition, Table, TableRow, Text, TextBlock,
        },
        text::WeightedTextBlock,
    },
    media::{image::Image, printer::RegisterImageError, register::ImageRegistry},
    presentation::{
        ChunkMutator, ImageProperties, MarginProperties, Modals, PreformattedLine, Presentation, PresentationMetadata,
        PresentationState, PresentationThemeMetadata, RenderOperation, Slide, SlideBuilder, SlideChunk,
    },
    processing::{
        code::{CodePreparer, HighlightContext, HighlightMutator, HighlightedLine},
        execution::RunCodeOperation,
        footer::{FooterContext, FooterGenerator},
        modals::IndexBuilder,
        separator::RenderSeparator,
    },
    render::highlighting::{CodeHighlighter, HighlightThemeSet},
    resource::{LoadImageError, Resources},
    style::{Color, Colors, TextStyle},
    theme::{
        Alignment, AuthorPositioning, ElementType, LoadThemeError, Margin, PresentationTheme, PresentationThemeSet,
    },
    typst::{TypstRender, TypstRenderError},
};
use image::DynamicImage;
use serde::Deserialize;
use std::{borrow::Cow, cell::RefCell, fmt::Display, iter, mem, path::PathBuf, rc::Rc, str::FromStr};
use unicode_width::UnicodeWidthStr;

use super::modals::KeyBindingsModalBuilder;

// TODO: move to a theme config.
static DEFAULT_BOTTOM_SLIDE_MARGIN: u16 = 3;
static DEFAULT_Z_INDEX: i32 = -2;

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
    pub incremental_lists: bool,
    pub force_default_theme: bool,
    pub end_slide_shorthand: bool,
    pub print_modal_background: bool,
    pub strict_front_matter_parsing: bool,
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
    }
}

impl Default for PresentationBuilderOptions {
    fn default() -> Self {
        Self {
            allow_mutations: true,
            implicit_slide_ends: false,
            command_prefix: String::default(),
            incremental_lists: false,
            force_default_theme: false,
            end_slide_shorthand: false,
            print_modal_background: false,
            strict_front_matter_parsing: true,
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
    theme: Cow<'a, PresentationTheme>,
    resources: &'a mut Resources,
    typst: &'a mut TypstRender,
    slide_state: SlideState,
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
        typst: &'a mut TypstRender,
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
            theme: Cow::Borrowed(default_theme),
            resources,
            typst,
            slide_state: Default::default(),
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
            self.process_element(element)?;
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

        let presentation_state = PresentationState::default();
        let slide_index = self.index_builder.build(&self.theme, presentation_state.clone());
        let bindings = bindings_modal_builder.build(&self.theme, &self.bindings_config);
        let modals = Modals { slide_index, bindings };
        let presentation = Presentation::new(self.slides, modals, presentation_state);
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
        // If we don't have an rgb color (or we don't have a color at all), we default to a semi
        // transparent dark background.
        let rgba = match color {
            Some((r, g, b)) => [r, g, b, 230],
            None => [0, 0, 0, 128],
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
        let colors = self.theme.default_style.colors.clone();
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

    fn process_element(&mut self, element: MarkdownElement) -> Result<(), BuildError> {
        let should_clear_last = !matches!(element, MarkdownElement::List(_) | MarkdownElement::Comment { .. });
        match element {
            // This one is processed before everything else as it affects how the rest of the
            // elements is rendered.
            MarkdownElement::FrontMatter(_) => self.slide_state.ignore_element_line_break = true,
            MarkdownElement::SetexHeading { text } => self.push_slide_title(text),
            MarkdownElement::Heading { level, text } => self.push_heading(level, text),
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements)?,
            MarkdownElement::List(elements) => self.push_list(elements),
            MarkdownElement::Code(code) => self.push_code(code)?,
            MarkdownElement::Table(table) => self.push_table(table),
            MarkdownElement::ThematicBreak => self.process_thematic_break(),
            MarkdownElement::Comment { comment, source_position } => self.process_comment(comment, source_position)?,
            MarkdownElement::BlockQuote(lines) => self.push_block_quote(lines),
            MarkdownElement::Image { path, .. } => self.push_image_from_path(path)?,
        };
        if should_clear_last {
            self.slide_state.last_element = LastElement::Other;
        }
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
        self.footer_context.borrow_mut().author = metadata.author.clone().unwrap_or_default();
        self.set_theme(&metadata.theme)?;
        if metadata.title.is_some()
            || metadata.sub_title.is_some()
            || metadata.author.is_some()
            || !metadata.authors.is_empty()
        {
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
        let styles = &self.theme.intro_slide;
        let title = Text::new(
            metadata.title.unwrap_or_default().clone(),
            TextStyle::default().bold().colors(styles.title.colors.clone()),
        );
        let sub_title = metadata
            .sub_title
            .as_ref()
            .map(|text| Text::new(text.clone(), TextStyle::default().colors(styles.subtitle.colors.clone())));
        let authors: Vec<_> = metadata
            .author
            .into_iter()
            .chain(metadata.authors)
            .map(|author| Text::new(author, TextStyle::default().colors(styles.author.colors.clone())))
            .collect();
        if styles.footer == Some(false) {
            self.slide_state.ignore_footer = true;
        }
        self.chunk_operations.push(RenderOperation::JumpToVerticalCenter);
        self.push_text(TextBlock::from(title), ElementType::PresentationTitle);
        self.push_line_break();
        if let Some(text) = sub_title {
            self.push_text(TextBlock::from(text), ElementType::PresentationSubTitle);
            self.push_line_break();
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
                self.push_text(TextBlock::from(author), ElementType::PresentationAuthor);
                self.push_line_break();
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
        let mut text_style = TextStyle::default().colors(style.colors.clone());
        if style.bold {
            text_style = text_style.bold();
        }
        if style.italics {
            text_style = text_style.italics();
        }
        if style.underlined {
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
        let text_style = TextStyle::default().bold().colors(style.colors.clone());
        text.apply_style(&text_style);

        self.push_text(text, element_type);
        self.push_line_break();
    }

    fn push_paragraph(&mut self, elements: Vec<ParagraphElement>) -> Result<(), BuildError> {
        for element in elements {
            match element {
                ParagraphElement::Text(text) => {
                    self.push_text(text, ElementType::Paragraph);
                    self.push_line_break();
                }
                ParagraphElement::LineBreak => {
                    // Line breaks are already pushed after every text chunk.
                }
            };
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

    fn push_image_from_path(&mut self, path: PathBuf) -> Result<(), BuildError> {
        let image = self.resources.image(&path)?;
        self.push_image(image);
        Ok(())
    }

    fn push_image(&mut self, image: Image) {
        let properties = ImageProperties {
            z_index: DEFAULT_Z_INDEX,
            size: Default::default(),
            restore_cursor: false,
            background_color: self.theme.default_style.colors.background,
        };
        self.chunk_operations.extend([
            RenderOperation::RenderImage(image, properties),
            RenderOperation::SetColors(self.theme.default_style.colors.clone()),
        ]);
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

    fn push_block_quote(&mut self, lines: Vec<String>) {
        let prefix = self.theme.block_quote.prefix.clone().unwrap_or_default();
        let block_length = lines.iter().map(|line| line.width() + prefix.width()).max().unwrap_or(0) as u16;
        let prefix_color = self.theme.block_quote.colors.prefix.or(self.theme.block_quote.colors.base.foreground);
        let prefix = Text::new(
            prefix,
            TextStyle::default()
                .colors(Colors { foreground: prefix_color, background: self.theme.block_quote.colors.base.background }),
        );
        let alignment = self.theme.alignment(&ElementType::BlockQuote).clone();
        let style = TextStyle::default().colors(self.theme.block_quote.colors.base.clone());

        for line in lines {
            let line = TextBlock(vec![prefix.clone(), Text::new(line, style.clone())]);
            self.chunk_operations.extend([
                // Print a preformatted empty block so we fill in the line with properly colored
                // spaces.
                RenderOperation::SetColors(self.theme.block_quote.colors.base.clone()),
                RenderOperation::RenderPreformattedLine(PreformattedLine {
                    text: "".into(),
                    unformatted_length: 0,
                    block_length,
                    alignment: alignment.clone(),
                }),
                // Now render our prefix + entire line
                RenderOperation::RenderText { line: line.into(), alignment: alignment.clone() },
            ]);
            self.push_line_break();
        }
        self.chunk_operations.push(RenderOperation::SetColors(self.theme.default_style.colors.clone()));
    }

    fn push_text(&mut self, text: TextBlock, element_type: ElementType) {
        let alignment = self.theme.alignment(&element_type);
        self.push_aligned_text(text, alignment);
    }

    fn push_aligned_text(&mut self, mut block: TextBlock, alignment: Alignment) {
        for chunk in &mut block.0 {
            if chunk.style.is_code() {
                chunk.style.colors = self.theme.inline_code.colors.clone();
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

    fn push_code(&mut self, code: Code) -> Result<(), BuildError> {
        if code.attributes.auto_render {
            return self.push_rendered_code(code);
        }
        let (lines, context) = self.highlight_lines(&code);
        for line in lines {
            self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(line)));
        }
        self.chunk_operations.push(RenderOperation::SetColors(self.theme.default_style.colors.clone()));
        if self.options.allow_mutations && context.borrow().groups.len() > 1 {
            self.chunk_mutators.push(Box::new(HighlightMutator::new(context)));
        }
        if code.attributes.execute {
            self.push_code_execution(code);
        }
        Ok(())
    }

    fn push_rendered_code(&mut self, code: Code) -> Result<(), BuildError> {
        let image = match code.language {
            CodeLanguage::Typst => self.typst.render_typst(&code.contents, &self.theme.typst)?,
            CodeLanguage::Latex => self.typst.render_latex(&code.contents, &self.theme.typst)?,
            _ => panic!("language {:?} should not be renderable", code.language),
        };
        self.push_image(image);
        Ok(())
    }

    fn highlight_lines(&self, code: &Code) -> (Vec<HighlightedLine>, Rc<RefCell<HighlightContext>>) {
        let lines = CodePreparer::new(&self.theme).prepare(code);
        let block_length = lines.iter().map(|line| line.width()).max().unwrap_or(0);
        let mut empty_highlighter = self.highlighter.language_highlighter(&CodeLanguage::Unknown);
        let mut code_highlighter = self.highlighter.language_highlighter(&code.language);
        let padding_style = {
            let mut highlighter = self.highlighter.language_highlighter(&CodeLanguage::Rust);
            highlighter.style_line("//").next().expect("no styles").style
        };
        let groups = match self.options.allow_mutations {
            true => code.attributes.highlight_groups.clone(),
            false => vec![HighlightGroup::new(vec![Highlight::All])],
        };
        let context = Rc::new(RefCell::new(HighlightContext {
            groups,
            current: 0,
            block_length,
            alignment: self.theme.alignment(&ElementType::Code),
        }));

        let mut output = Vec::new();
        let block_style = &self.theme.code;
        for line in lines.into_iter() {
            let highlighted = line.highlight(&padding_style, &mut code_highlighter, block_style);
            let not_highlighted = line.highlight(&padding_style, &mut empty_highlighter, block_style);
            let width = line.width();
            let line_number = line.line_number;
            let context = context.clone();
            output.push(HighlightedLine { highlighted, not_highlighted, line_number, width, context });
        }
        (output, context)
    }

    fn push_code_execution(&mut self, code: Code) {
        let operation = RunCodeOperation::new(
            code,
            self.theme.default_style.colors.clone(),
            self.theme.execution_output.colors.clone(),
        );
        let operation = RenderOperation::RenderOnDemand(Rc::new(operation));
        self.chunk_operations.push(operation);
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
    #[error("loading image: {0}")]
    LoadImage(#[from] LoadImageError),

    #[error("registering image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("invalid presentation metadata: {0}")]
    InvalidMetadata(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(#[from] LoadThemeError),

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

    #[error("typst render failed: {0}")]
    TypstRender(#[from] TypstRenderError),
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
        let StrictPresentationMetadata { title, sub_title, author, authors, theme, options } = strict;
        Self { title, sub_title, author, authors, theme, options }
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
        let mut typst = TypstRender::default();
        let themes = Themes::default();
        let bindings = KeyBindingsConfig::default();
        let builder = PresentationBuilder::new(
            &theme,
            &mut resources,
            &mut typst,
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
            | RenderPreformattedLine(_)
            | RenderDynamic(_)
            | RenderOnDemand(_) => true,
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
                slide.iter_operations().filter(|op| matches!(op, RenderOperation::ClearScreen)).count();
            let set_colors_count =
                slide.iter_operations().filter(|op| matches!(op, RenderOperation::SetColors(_))).count();
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
        let operations = first_chunk.iter_operations().collect::<Vec<_>>();
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
            MarkdownElement::Paragraph(vec![ParagraphElement::Text("hi".into())]),
        ];
        let presentation = build_presentation_with_options(elements, options);
        assert_eq!(presentation.iter_slides().count(), 2);

        let second = presentation.iter_slides().nth(1).unwrap();
        let before_text = second.iter_operations().take_while(|e| !matches!(e, RenderOperation::RenderText { .. }));
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
}
