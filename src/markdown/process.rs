use crate::{
    markdown::elements::{
        Code, FormattedText, ListItem, ListItemType, MarkdownElement, ParagraphElement, TableRow, Text, TextChunk,
        TextFormat,
    },
    presentation::{RenderOperation, Slide},
    render::highlighting::{CodeHighlighter, CodeLine},
    resource::{LoadImageError, Resources},
    theme::{AuthorPositioning, ElementType, SlideTheme},
};
use std::{iter, mem};

use super::{
    elements::{PresentationMetadata, Table},
    text::{WeightedLine, WeightedText},
};

pub type ProcessError = LoadImageError;

pub struct MarkdownProcessor<'a> {
    slide_operations: Vec<RenderOperation>,
    slides: Vec<Slide>,
    highlighter: &'a CodeHighlighter,
    theme: &'a SlideTheme,
    resources: &'a mut Resources,
}

impl<'a> MarkdownProcessor<'a> {
    pub fn new(highlighter: &'a CodeHighlighter, theme: &'a SlideTheme, resources: &'a mut Resources) -> Self {
        Self { slide_operations: Vec::new(), slides: Vec::new(), highlighter, theme, resources }
    }

    pub fn transform(mut self, elements: Vec<MarkdownElement>) -> Result<Vec<Slide>, LoadImageError> {
        for element in elements {
            self.process_element(element)?;
            self.push_line_break();
        }
        if !self.slide_operations.is_empty() {
            self.terminate_slide();
        }
        Ok(self.slides)
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
        };
        Ok(())
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) {
        let title = FormattedText::formatted(metadata.title.clone(), TextFormat::default().add_bold());
        let sub_title = metadata.sub_title.as_ref().map(|text| FormattedText::plain(text.clone()));
        let author = metadata.author.as_ref().map(|text| FormattedText::plain(text.clone()));
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

    fn push_slide_title(&mut self, mut text: Text) {
        text.apply_format(&TextFormat::default().add_bold());

        self.push_line_break();
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();
        self.push_line_break();
        self.slide_operations.push(RenderOperation::RenderSeparator);
        self.push_line_break();
    }

    fn push_heading(&mut self, level: u8, mut text: Text) {
        let element_type = match level {
            1 => ElementType::Heading1,
            2 => ElementType::Heading2,
            3 => ElementType::Heading3,
            4 => ElementType::Heading4,
            5 => ElementType::Heading5,
            6 => ElementType::Heading6,
            other => panic!("unexpected heading level {other}"),
        };
        text.apply_format(&TextFormat::default().add_bold());
        self.push_text(text, element_type);
        self.push_line_break();
    }

    fn push_paragraph(&mut self, elements: Vec<ParagraphElement>) -> Result<(), ProcessError> {
        for element in elements {
            match element {
                ParagraphElement::Text(mut text) => {
                    // TODO: 2?
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
        let mut text = item.contents.clone();
        text.chunks.insert(0, TextChunk::Formatted(FormattedText::plain(prefix)));
        self.push_text(text, ElementType::List);
        self.push_line_break();
    }

    fn push_text(&mut self, text: Text, element_type: ElementType) {
        let mut texts: Vec<WeightedText> = Vec::new();
        for chunk in text.chunks {
            match chunk {
                TextChunk::Formatted(text) => {
                    texts.push(text.into());
                }
                TextChunk::LineBreak => {
                    if !texts.is_empty() {
                        self.slide_operations.push(RenderOperation::RenderTextLine {
                            texts: WeightedLine::from(mem::take(&mut texts)),
                            element_type: element_type.clone(),
                        });
                    }
                    self.push_line_break()
                }
            }
        }
        if !texts.is_empty() {
            self.slide_operations.push(RenderOperation::RenderTextLine {
                texts: WeightedLine::from(texts),
                element_type: element_type.clone(),
            });
        }
    }

    fn push_line_break(&mut self) {
        self.slide_operations.push(RenderOperation::RenderLineBreak);
    }

    fn push_code(&mut self, code: Code) {
        let block_length = code.contents.lines().map(|line| line.len()).max().unwrap_or(0);
        for code_line in self.highlighter.highlight(&code.contents, &code.language) {
            let CodeLine { formatted, original } = code_line;
            let formatted = formatted.trim_end();
            self.slide_operations.push(RenderOperation::RenderPreformattedLine {
                text: formatted.into(),
                // TODO: remove once measuring character widths is in place
                original_length: original.len(),
                block_length,
            });
            self.push_line_break();
        }
    }

    fn terminate_slide(&mut self) {
        let elements = mem::take(&mut self.slide_operations);
        self.slides.push(Slide { render_operations: elements });
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
            separator.chunks.push(TextChunk::Formatted(FormattedText::plain(contents)));
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
                flattened_row.chunks.push(TextChunk::Formatted(FormattedText::plain(" │ ")));
            }
            let text_length = text.line_len();
            flattened_row.chunks.extend(text.chunks.into_iter());

            let cell_width = widths[column];
            if text_length < cell_width {
                let padding = " ".repeat(cell_width - text_length);
                flattened_row.chunks.push(TextChunk::Formatted(FormattedText::plain(padding)));
            }
        }
        flattened_row
    }
}
