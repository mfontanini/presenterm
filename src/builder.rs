use crate::{
    markdown::{
        elements::{
            Code, ListItem, ListItemType, MarkdownElement, ParagraphElement, StyledText, Table, TableRow, Text,
            TextChunk,
        },
        text::{WeightedLine, WeightedText},
    },
    presentation::{
        AsRenderOperations, Presentation, PresentationMetadata, PresentationThemeMetadata, RenderOperation, Slide,
    },
    render::highlighting::{CodeHighlighter, CodeLine},
    resource::{LoadImageError, Resources},
    style::TextStyle,
    theme::{Alignment, AuthorPositioning, ElementType, FooterStyle, LoadThemeError, PresentationTheme},
};
use std::{borrow::Cow, cell::RefCell, iter, mem, rc::Rc, str::FromStr};
use unicode_width::UnicodeWidthStr;

pub struct PresentationBuilder<'a> {
    slide_operations: Vec<RenderOperation>,
    slides: Vec<Slide>,
    highlighter: &'a CodeHighlighter,
    theme: Cow<'a, PresentationTheme>,
    resources: &'a mut Resources,
    ignore_element_line_break: bool,
    last_element_is_list: bool,
    footer_context: Rc<RefCell<FooterContext>>,
}

impl<'a> PresentationBuilder<'a> {
    pub fn new(
        highlighter: &'a CodeHighlighter,
        default_theme: &'a PresentationTheme,
        resources: &'a mut Resources,
    ) -> Self {
        Self {
            slide_operations: Vec::new(),
            slides: Vec::new(),
            highlighter,
            theme: Cow::Borrowed(default_theme),
            resources,
            ignore_element_line_break: false,
            last_element_is_list: false,
            footer_context: Default::default(),
        }
    }

    pub fn build(mut self, elements: Vec<MarkdownElement>) -> Result<Presentation, BuildError> {
        if let Some(MarkdownElement::FrontMatter(contents)) = elements.first() {
            self.process_front_matter(contents)?;
        }
        self.push_slide_prelude();
        for element in elements {
            self.ignore_element_line_break = false;
            self.process_element(element)?;
            if !self.ignore_element_line_break {
                self.push_line_break();
            }
        }
        if !self.slide_operations.is_empty() {
            self.terminate_slide();
        }
        self.footer_context.borrow_mut().total_slides = self.slides.len();

        let presentation = Presentation::new(self.slides);
        Ok(presentation)
    }

    fn push_slide_prelude(&mut self) {
        let colors = self.theme.default_style.colors.clone();
        self.slide_operations.push(RenderOperation::SetColors(colors));
        self.slide_operations.push(RenderOperation::ClearScreen);
    }

    fn process_element(&mut self, element: MarkdownElement) -> Result<(), BuildError> {
        let is_list = matches!(element, MarkdownElement::List(_));
        match element {
            // This one is processed before everything else as it affects how the rest of the
            // elements is rendered.
            MarkdownElement::FrontMatter(_) => (),
            MarkdownElement::SlideTitle { text } => self.push_slide_title(text),
            MarkdownElement::Heading { level, text } => self.push_heading(level, text),
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements)?,
            MarkdownElement::List(elements) => self.push_list(elements),
            MarkdownElement::Code(code) => self.push_code(code),
            MarkdownElement::Table(table) => self.push_table(table),
            MarkdownElement::ThematicBreak => self.terminate_slide(),
            MarkdownElement::Comment(comment) => self.process_comment(comment),
        };
        self.last_element_is_list = is_list;
        Ok(())
    }

    fn process_front_matter(&mut self, contents: &str) -> Result<(), BuildError> {
        let metadata: PresentationMetadata =
            serde_yaml::from_str(contents).map_err(|e| BuildError::InvalidMetadata(e.to_string()))?;

        self.footer_context.borrow_mut().author = metadata.author.clone().unwrap_or_default();
        self.set_theme(&metadata.theme)?;
        if metadata.title.is_some() || metadata.sub_title.is_some() || metadata.author.is_some() {
            self.push_slide_prelude();
            self.push_intro_slide(metadata);
        }
        Ok(())
    }

    fn set_theme(&mut self, metadata: &PresentationThemeMetadata) -> Result<(), BuildError> {
        if metadata.theme_name.is_some() && metadata.theme_path.is_some() {
            return Err(BuildError::InvalidMetadata("cannot have both theme path and theme name".into()));
        }
        if let Some(theme_name) = &metadata.theme_name {
            let theme = PresentationTheme::from_name(theme_name)
                .ok_or_else(|| BuildError::InvalidMetadata(format!("theme '{theme_name}' does not exist")))?;
            self.theme = Cow::Owned(theme);
        }
        if let Some(theme_path) = &metadata.theme_path {
            let theme = PresentationTheme::from_path(theme_path)?;
            self.theme = Cow::Owned(theme);
        }
        if let Some(overrides) = &metadata.overrides {
            // This shouldn't fail as the models are already correct.
            let theme = merge_struct::merge(self.theme.as_ref(), overrides)
                .map_err(|_| BuildError::InvalidMetadata("invalid theme".to_string()))?;
            self.theme = Cow::Owned(theme);
        }
        Ok(())
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) {
        let title = StyledText::styled(metadata.title.unwrap_or_default().clone(), TextStyle::default().bold());
        let sub_title = metadata.sub_title.as_ref().map(|text| StyledText::plain(text.clone()));
        let author = metadata.author.as_ref().map(|text| StyledText::plain(text.clone()));
        self.slide_operations.push(RenderOperation::JumpToVerticalCenter);
        self.push_text(Text::single(title), ElementType::PresentationTitle);
        self.push_line_break();
        if let Some(text) = sub_title {
            self.push_text(Text::single(text), ElementType::PresentationSubTitle);
            self.push_line_break();
        }
        if let Some(text) = author {
            match self.theme.presentation.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.push_line_break();
                    self.push_line_break();
                    self.push_line_break();
                }
                AuthorPositioning::PageBottom => {
                    self.slide_operations.push(RenderOperation::JumpToSlideBottom);
                }
            };
            self.push_text(Text::single(text), ElementType::PresentationAuthor);
        }
        self.terminate_slide();
    }

    fn process_comment(&mut self, comment: String) {
        let Ok(comment) = comment.parse::<Comment>() else { return; };
        match comment {
            Comment::Pause => self.process_pause(),
            Comment::EndSlide => self.terminate_slide(),
        }
    }

    fn process_pause(&mut self) {
        // Remove the last line, if any, if the previous element is a list. This allows each
        // element in a list showing up without newlines in between..
        if self.last_element_is_list && matches!(self.slide_operations.last(), Some(RenderOperation::RenderLineBreak)) {
            self.slide_operations.pop();
        }

        let next_operations = self.slide_operations.clone();
        self.terminate_slide();
        self.slide_operations = next_operations;
        self.ignore_element_line_break = true;
    }

    fn push_slide_title(&mut self, mut text: Text) {
        let style = self.theme.slide_title.clone();
        text.apply_style(&TextStyle::default().bold().colors(style.colors.clone()));

        for _ in 0..style.padding_top.unwrap_or(0) {
            self.push_line_break();
        }
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();

        for _ in 0..style.padding_bottom.unwrap_or(0) {
            self.push_line_break();
        }
        if style.separator {
            self.slide_operations.push(RenderOperation::RenderSeparator);
        }
        self.push_line_break();
    }

    fn push_heading(&mut self, level: u8, mut text: Text) {
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
            text.chunks.insert(0, TextChunk::Styled(StyledText::plain(prefix)));
        }
        let text_style = TextStyle::default().bold().colors(style.colors.clone());
        text.apply_style(&text_style);

        self.push_text(text, element_type);
        self.push_line_break();
    }

    fn push_paragraph(&mut self, elements: Vec<ParagraphElement>) -> Result<(), BuildError> {
        for element in elements {
            match element {
                ParagraphElement::Text(mut text) => {
                    text.chunks.push(TextChunk::LineBreak);
                    self.push_text(text, ElementType::Paragraph);
                }
                ParagraphElement::Image { url } => {
                    let image = self.resources.image(&url)?;
                    self.slide_operations.push(RenderOperation::RenderImage(image));
                }
            };
        }
        Ok(())
    }

    fn push_list(&mut self, items: Vec<ListItem>) {
        for item in items {
            self.push_list_item(item);
        }
    }

    fn push_list_item(&mut self, item: ListItem) {
        let padding_length = (item.depth as usize + 1) * 2;
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
            ListItemType::OrderedParens(number) => {
                prefix.push_str(&number.to_string());
                prefix.push_str(") ");
            }
            ListItemType::OrderedPeriod(number) => {
                prefix.push_str(&number.to_string());
                prefix.push_str(". ");
            }
        };

        prefix.push(' ');
        let mut text = item.contents;
        text.chunks.insert(0, TextChunk::Styled(StyledText::plain(prefix)));
        self.push_text(text, ElementType::List);
        self.push_line_break();
    }

    fn push_text(&mut self, text: Text, element_type: ElementType) {
        let alignment = self.theme.alignment(&element_type);
        let mut texts: Vec<WeightedText> = Vec::new();
        for chunk in text.chunks {
            match chunk {
                TextChunk::Styled(mut text) => {
                    if text.style.is_code() {
                        text.style.colors = self.theme.code.colors.clone();
                    }
                    texts.push(text.into());
                }
                TextChunk::LineBreak => {
                    if !texts.is_empty() {
                        self.slide_operations.push(RenderOperation::RenderTextLine {
                            texts: WeightedLine::from(mem::take(&mut texts)),
                            alignment: alignment.clone(),
                        });
                    }
                    self.push_line_break()
                }
            }
        }
        if !texts.is_empty() {
            self.slide_operations.push(RenderOperation::RenderTextLine {
                texts: WeightedLine::from(texts),
                alignment: alignment.clone(),
            });
        }
    }

    fn push_line_break(&mut self) {
        self.slide_operations.push(RenderOperation::RenderLineBreak);
    }

    fn push_code(&mut self, code: Code) {
        let Code { contents, language } = code;
        let mut code = String::new();
        let horizontal_padding = self.theme.code.padding.horizontal.unwrap_or(0);
        let vertical_padding = self.theme.code.padding.vertical.unwrap_or(0);
        if horizontal_padding == 0 && vertical_padding == 0 {
            code = contents;
        } else {
            if vertical_padding > 0 {
                code.push('\n');
            }
            if horizontal_padding > 0 {
                let padding = " ".repeat(horizontal_padding as usize);
                for line in contents.lines() {
                    code.push_str(&padding);
                    code.push_str(line);
                    code.push('\n');
                }
            } else {
                code.push_str(&contents);
            }
            if vertical_padding > 0 {
                code.push('\n');
            }
        }
        let block_length = code.lines().map(|line| line.len()).max().unwrap_or(0) + horizontal_padding as usize;
        for code_line in self.highlighter.highlight(&code, &language) {
            let CodeLine { formatted, original } = code_line;
            let trimmed = formatted.trim_end();
            let original_length = original.len() - (formatted.len() - trimmed.len());
            self.slide_operations.push(RenderOperation::RenderPreformattedLine {
                text: trimmed.into(),
                unformatted_length: original_length,
                block_length,
                alignment: self.theme.alignment(&ElementType::Code).clone(),
            });
            self.push_line_break();
        }
    }

    fn terminate_slide(&mut self) {
        self.push_footer();

        let elements = mem::take(&mut self.slide_operations);
        self.slides.push(Slide { render_operations: elements });
        self.push_slide_prelude();
    }

    fn push_footer(&mut self) {
        let generator = FooterGenerator {
            style: self.theme.footer.clone(),
            current_slide: self.slides.len(),
            context: self.footer_context.clone(),
        };
        self.slide_operations.push(RenderOperation::RenderDynamic(Rc::new(generator)));
    }

    fn push_table(&mut self, table: Table) {
        let widths: Vec<_> = (0..table.columns())
            .map(|column| table.iter_column(column).map(|text| text.line_len()).max().unwrap_or(0))
            .collect();
        let flattened_header = Self::prepare_table_row(table.header, &widths);
        self.push_text(flattened_header, ElementType::Table);
        self.push_line_break();

        let mut separator = Text { chunks: Vec::new() };
        for (index, width) in widths.iter().enumerate() {
            let mut contents = String::new();
            let mut extra_lines = 1;
            if index > 0 {
                contents.push('┼');
                extra_lines += 1;
            }
            contents.extend(iter::repeat("─").take(*width + extra_lines));
            separator.chunks.push(TextChunk::Styled(StyledText::plain(contents)));
        }

        self.push_text(separator, ElementType::Table);
        self.push_line_break();

        for row in table.rows {
            let flattened_row = Self::prepare_table_row(row, &widths);
            self.push_text(flattened_row, ElementType::Table);
            self.push_line_break();
        }
    }

    fn prepare_table_row(row: TableRow, widths: &[usize]) -> Text {
        let mut flattened_row = Text { chunks: Vec::new() };
        for (column, text) in row.0.into_iter().enumerate() {
            if column > 0 {
                flattened_row.chunks.push(TextChunk::Styled(StyledText::plain(" │ ")));
            }
            let text_length = text.line_len();
            flattened_row.chunks.extend(text.chunks.into_iter());

            let cell_width = widths[column];
            if text_length < cell_width {
                let padding = " ".repeat(cell_width - text_length);
                flattened_row.chunks.push(TextChunk::Styled(StyledText::plain(padding)));
            }
        }
        flattened_row
    }
}

#[derive(Debug, Default)]
struct FooterContext {
    total_slides: usize,
    author: String,
}

#[derive(Debug)]
struct FooterGenerator {
    current_slide: usize,
    context: Rc<RefCell<FooterContext>>,
    style: FooterStyle,
}

impl FooterGenerator {
    fn render_template(template: &str, current_slide: &str, context: &FooterContext) -> String {
        template
            .replace("{current_slide}", current_slide)
            .replace("{total_slides}", &context.total_slides.to_string())
            .replace("{author}", &context.author)
    }
}

impl AsRenderOperations for FooterGenerator {
    fn as_render_operations(&self, dimensions: &crossterm::terminal::WindowSize) -> Vec<RenderOperation> {
        let context = self.context.borrow();
        match &self.style {
            FooterStyle::Template { left, right } => {
                let current_slide = (self.current_slide + 1).to_string();
                let mut operations = Vec::new();
                if let Some(left) = left {
                    operations.extend([
                        RenderOperation::JumpToWindowBottom,
                        RenderOperation::RenderTextLine {
                            texts: Self::render_template(left, &current_slide, &context).into(),
                            alignment: Alignment::Left { margin: 1 },
                        },
                    ]);
                }
                if let Some(right) = right {
                    operations.extend([
                        RenderOperation::JumpToWindowBottom,
                        RenderOperation::RenderTextLine {
                            texts: Self::render_template(right, &current_slide, &context).into(),
                            alignment: Alignment::Right { margin: 1 },
                        },
                    ]);
                }
                operations
            }
            FooterStyle::ProgressBar { character } => {
                let character = character.unwrap_or('█').to_string();
                let total_columns = dimensions.columns as usize / character.width();
                let progress_ratio = (self.current_slide + 1) as f64 / context.total_slides as f64;
                let columns_ratio = (total_columns as f64 * progress_ratio).ceil();
                let bar = character.repeat(columns_ratio as usize);
                vec![
                    RenderOperation::JumpToWindowBottom,
                    RenderOperation::RenderTextLine { texts: bar.into(), alignment: Alignment::Left { margin: 0 } },
                ]
            }
            FooterStyle::Empty => vec![],
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("loading image: {0}")]
    LoadImage(#[from] LoadImageError),

    #[error("invalid presentation metadata: {0}")]
    InvalidMetadata(String),

    #[error("invalid theme: {0}")]
    InvalidTheme(#[from] LoadThemeError),
}

enum Comment {
    Pause,
    EndSlide,
}

impl FromStr for Comment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pause" => Ok(Self::Pause),
            "end_slide" => Ok(Self::EndSlide),
            _ => Err(()),
        }
    }
}
