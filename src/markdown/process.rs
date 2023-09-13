use crate::{
    markdown::elements::{
        Code, FormattedText, ListItem, ListItemType, MarkdownElement, ParagraphElement, PresentationMetadata, TableRow,
        Text, TextChunk, TextFormat,
    },
    render::highlighting::{CodeHighlighter, CodeLine},
    theme::ElementType,
};
use std::{iter, mem};

pub struct MarkdownProcessor<'a> {
    slide_elements: Vec<SlideElement>,
    slides: Vec<Slide>,
    highlighter: &'a CodeHighlighter,
}

impl<'a> MarkdownProcessor<'a> {
    pub fn new(highlighter: &'a CodeHighlighter) -> Self {
        Self { slide_elements: Vec::new(), slides: Vec::new(), highlighter }
    }

    pub fn transform(mut self, elements: Vec<MarkdownElement>) -> Vec<Slide> {
        for element in elements {
            self.process_element(element);
            self.push_line_break();
        }
        if !self.slide_elements.is_empty() {
            self.terminate_slide();
        }
        self.slides
    }

    fn process_element(&mut self, element: MarkdownElement) {
        match element {
            MarkdownElement::PresentationMetadata(metadata) => {
                self.slide_elements.push(SlideElement::PresentationMetadata(metadata));
                self.terminate_slide();
            }
            MarkdownElement::SlideTitle { text } => self.push_slide_title(text),
            MarkdownElement::Heading { level, text } => self.push_heading(level, text),
            MarkdownElement::Paragraph(elements) => self.push_paragraph(elements),
            MarkdownElement::List(elements) => self.push_list(elements),
            MarkdownElement::Code(code) => self.push_code(code),
            MarkdownElement::Table { header, rows } => self.push_table(header, rows),
            MarkdownElement::ThematicBreak => self.terminate_slide(),
        }
    }

    fn push_slide_title(&mut self, mut text: Text) {
        text.apply_format(&TextFormat::default().add_bold());

        self.push_line_break();
        self.push_text(text, ElementType::SlideTitle);
        self.push_line_break();
        self.push_line_break();
        self.slide_elements.push(SlideElement::Separator);
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

    fn push_paragraph(&mut self, elements: Vec<ParagraphElement>) {
        for element in elements {
            match element {
                ParagraphElement::Text(mut text) => {
                    // TODO: 2?
                    text.chunks.push(TextChunk::LineBreak);
                    self.push_text(text, ElementType::Paragraph);
                }
                ParagraphElement::Image { url } => self.slide_elements.push(SlideElement::Image { url }),
            };
        }
    }

    fn push_list(&mut self, items: Vec<ListItem>) {
        for item in items {
            self.transform_list_item(item);
        }
    }

    fn transform_list_item(&mut self, item: ListItem) {
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
        // TODO move line break outside of TextChunk
        let mut texts = Vec::new();
        for chunk in text.chunks {
            match chunk {
                TextChunk::Formatted(text) => {
                    texts.push(text);
                }
                TextChunk::LineBreak => {
                    if !texts.is_empty() {
                        self.slide_elements.push(SlideElement::TextLine {
                            texts: mem::take(&mut texts),
                            element_type: element_type.clone(),
                        });
                    }
                    self.push_line_break()
                }
            }
        }
        if !texts.is_empty() {
            self.slide_elements.push(SlideElement::TextLine { texts, element_type: element_type.clone() });
        }
    }

    fn push_line_break(&mut self) {
        self.slide_elements.push(SlideElement::LineBreak);
    }

    fn push_code(&mut self, code: Code) {
        let block_length = code.contents.lines().map(|line| line.len()).max().unwrap_or(0);
        for code_line in self.highlighter.highlight(&code.contents, &code.language) {
            let CodeLine { formatted, original } = code_line;
            let formatted = formatted.trim_end();
            self.slide_elements.push(SlideElement::PreformattedLine {
                text: formatted.into(),
                // TODO: remove once measuring character widths is in place
                original_length: original.len(),
                block_length,
            });
            self.push_line_break();
        }
    }

    fn terminate_slide(&mut self) {
        let elements = mem::take(&mut self.slide_elements);
        self.slides.push(Slide { elements });
    }

    fn push_table(&mut self, header: TableRow, rows: Vec<TableRow>) {
        let widths = Self::calculate_table_column_width(&header, &rows);
        let flattened_header = Self::prepare_table_row(header, &widths);
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

        for row in rows {
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

    fn calculate_table_column_width(header: &TableRow, rows: &[TableRow]) -> Vec<usize> {
        let mut widths = Vec::new();
        for (column, header_element) in header.0.iter().enumerate() {
            let row_elements = rows.iter().map(|row| &row.0[column]);
            let max_width =
                iter::once(header_element).chain(row_elements).map(|text| text.line_len()).max().unwrap_or(0);
            widths.push(max_width);
        }
        widths
    }
}

#[derive(Clone, Debug)]
pub enum SlideElement {
    PresentationMetadata(PresentationMetadata),
    TextLine { texts: Vec<FormattedText>, element_type: ElementType },
    Separator,
    LineBreak,
    Image { url: String },
    PreformattedLine { text: String, original_length: usize, block_length: usize },
}

#[derive(Clone, Debug)]
pub struct Slide {
    pub elements: Vec<SlideElement>,
}

// #[cfg(test)]
// mod test {
//     use super::*;
//
//
// }
