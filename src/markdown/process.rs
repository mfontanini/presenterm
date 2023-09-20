use super::{
    elements::{PresentationMetadata, Table},
    text::{WeightedLine, WeightedText},
};
use crate::{
    markdown::elements::{
        Code, ListItem, ListItemType, MarkdownElement, ParagraphElement, StyledText, TableRow, Text, TextChunk,
    },
    presentation::{RenderOperation, Slide},
    render::highlighting::{CodeHighlighter, CodeLine},
    resource::{LoadImageError, Resources},
    style::TextStyle,
    theme::{AuthorPositioning, ElementType, PresentationTheme},
};
use std::{iter, mem};

pub type ProcessError = LoadImageError;

pub struct MarkdownProcessor<'a> {
    slide_operations: Vec<RenderOperation>,
    slides: Vec<Slide>,
    highlighter: &'a CodeHighlighter,
    theme: &'a PresentationTheme,
    resources: &'a mut Resources,
    ignore_element_line_break: bool,
}

impl<'a> MarkdownProcessor<'a> {
    pub fn new(highlighter: &'a CodeHighlighter, theme: &'a PresentationTheme, resources: &'a mut Resources) -> Self {
        Self {
            slide_operations: Vec::new(),
            slides: Vec::new(),
            highlighter,
            theme,
            resources,
            ignore_element_line_break: false,
        }
    }

    pub fn transform(mut self, elements: Vec<MarkdownElement>) -> Result<Vec<Slide>, LoadImageError> {
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
        Ok(self.slides)
    }

    fn push_slide_prelude(&mut self) {
        let colors = self.theme.styles.default_style.colors.clone();
        self.slide_operations.push(RenderOperation::SetColors(colors));
        self.slide_operations.push(RenderOperation::ClearScreen);
    }

    fn process_element(&mut self, element: MarkdownElement) -> Result<(), ProcessError> {
        match element {
            MarkdownElement::PresentationMetadata(metadata) => self.push_intro_slide(metadata),
            MarkdownElement::SlideTitle { text } => self.push_slide_title(text),
            MarkdownElement::Heading { level, text } => self.push_heading(level, text),
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements)?,
            MarkdownElement::List(elements) => self.push_list(elements),
            MarkdownElement::Code(code) => self.push_code(code),
            MarkdownElement::Table(table) => self.push_table(table),
            MarkdownElement::ThematicBreak => self.terminate_slide(),
            MarkdownElement::Comment(comment) => self.process_comment(comment),
        };
        Ok(())
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) {
        let title = StyledText::styled(metadata.title.clone(), TextStyle::default().bold());
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
            match self.theme.styles.presentation.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.push_line_break();
                    self.push_line_break();
                    self.push_line_break();
                }
                AuthorPositioning::PageBottom => {
                    self.slide_operations.push(RenderOperation::JumpToBottom);
                }
            };
            self.push_text(Text::single(text), ElementType::PresentationAuthor);
        }
        self.terminate_slide();
    }

    fn process_comment(&mut self, comment: String) {
        if comment != "pause" {
            return;
        }
        // Remove the last line break, if any.
        if matches!(self.slide_operations.last(), Some(RenderOperation::RenderLineBreak)) {
            self.slide_operations.pop();
        }

        let next_operations = self.slide_operations.clone();
        self.terminate_slide();
        self.slide_operations = next_operations;
        self.ignore_element_line_break = true;
    }

    fn push_slide_title(&mut self, mut text: Text) {
        text.apply_style(&TextStyle::default().bold());

        self.push_line_break();
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();
        self.push_line_break();
        self.slide_operations.push(RenderOperation::RenderSeparator);
        self.push_line_break();
    }

    fn push_heading(&mut self, level: u8, mut text: Text) {
        let (element_type, style) = match level {
            1 => (ElementType::Heading1, &self.theme.styles.headings.h1),
            2 => (ElementType::Heading2, &self.theme.styles.headings.h2),
            3 => (ElementType::Heading3, &self.theme.styles.headings.h3),
            4 => (ElementType::Heading4, &self.theme.styles.headings.h4),
            5 => (ElementType::Heading5, &self.theme.styles.headings.h5),
            6 => (ElementType::Heading6, &self.theme.styles.headings.h6),
            other => panic!("unexpected heading level {other}"),
        };
        if !style.prefix.is_empty() {
            let mut prefix = style.prefix.clone();
            prefix.push(' ');
            text.chunks.insert(0, TextChunk::Styled(StyledText::plain(prefix)));
        }
        let text_style = TextStyle::default().bold().colors(style.colors.clone());
        text.apply_style(&text_style);

        self.push_text(text, element_type);
        self.push_line_break();
    }

    fn push_paragraph(&mut self, elements: Vec<ParagraphElement>) -> Result<(), ProcessError> {
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
                        text.style.colors = self.theme.styles.code.colors.clone();
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
        let horizontal_padding = self.theme.styles.code.padding.horizontal;
        let vertical_padding = self.theme.styles.code.padding.vertical;
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
        let block_length = code.lines().map(|line| line.len()).max().unwrap_or(0);
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
        let elements = mem::take(&mut self.slide_operations);
        self.slides.push(Slide { render_operations: elements });
        self.push_slide_prelude();
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
