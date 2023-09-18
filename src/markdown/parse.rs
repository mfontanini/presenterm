use super::elements::Table;
use crate::{
    format::TextFormat,
    markdown::elements::{
        Code, CodeLanguage, FormattedText, ListItem, ListItemType, MarkdownElement, ParagraphElement, TableRow, Text,
        TextChunk,
    },
};
use comrak::{
    nodes::{AstNode, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeList, NodeValue},
    parse_document, Arena, ComrakOptions,
};
use std::mem;

type ParseResult<T> = Result<T, ParseError>;

pub struct ParserOptions(ComrakOptions);

impl Default for ParserOptions {
    fn default() -> Self {
        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".into());
        options.extension.table = true;
        options.extension.strikethrough = true;
        Self(options)
    }
}

pub struct MarkdownParser<'a> {
    arena: &'a Arena<AstNode<'a>>,
    options: ComrakOptions,
}

impl<'a> MarkdownParser<'a> {
    pub fn new(arena: &'a Arena<AstNode<'a>>) -> Self {
        Self { arena, options: ParserOptions::default().0 }
    }

    pub fn parse(&self, document: &str) -> ParseResult<Vec<MarkdownElement>> {
        let node = parse_document(self.arena, document, &self.options);
        let mut elements = Vec::new();
        for node in node.children() {
            let element = Self::parse_element(node)?;
            elements.push(element);
        }
        Ok(elements)
    }

    fn parse_element(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let value = &node.data.borrow().value;
        match value {
            NodeValue::FrontMatter(contents) => Self::parse_front_matter(contents),
            NodeValue::Heading(heading) => Self::parse_heading(heading, node),
            NodeValue::Paragraph => Self::parse_paragraph(node),
            NodeValue::List(_) => {
                let items = Self::parse_list(node, 0)?;
                Ok(MarkdownElement::List(items))
            }
            NodeValue::Table(_) => Self::parse_table(node),
            NodeValue::CodeBlock(block) => Self::parse_code_block(block),
            NodeValue::ThematicBreak => Ok(MarkdownElement::ThematicBreak),
            other => Err(ParseError::UnsupportedElement(other.identifier())),
        }
    }

    fn parse_front_matter(contents: &str) -> ParseResult<MarkdownElement> {
        // Remote leading and trailing delimiters before parsing. This is quite poopy but hey, it
        // works.
        let contents = contents.strip_prefix("---\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n\n").unwrap_or(contents);
        let title = serde_yaml::from_str(contents).map_err(|e| ParseError::InvalidMetadata(e.to_string()))?;
        let element = MarkdownElement::PresentationMetadata(title);
        Ok(element)
    }

    fn parse_code_block(block: &NodeCodeBlock) -> ParseResult<MarkdownElement> {
        if !block.fenced {
            return Err(ParseError::UnfencedCodeBlock);
        }
        let language = match block.info.as_str() {
            "rust" => CodeLanguage::Rust,
            "go" => CodeLanguage::Go,
            "c" => CodeLanguage::C,
            "cpp" => CodeLanguage::Cpp,
            "python" => CodeLanguage::Python,
            "typescript" | "ts" => CodeLanguage::Typescript,
            "javascript" | "js" => CodeLanguage::Javascript,
            _ => CodeLanguage::Unknown,
        };
        let code = Code { contents: block.literal.clone(), language };
        Ok(MarkdownElement::Code(code))
    }

    fn parse_heading(heading: &NodeHeading, node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let text = Self::parse_text(node)?;
        if heading.setext {
            Ok(MarkdownElement::SlideTitle { text })
        } else {
            Ok(MarkdownElement::Heading { text, level: heading.level })
        }
    }

    fn parse_paragraph(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let inlines = Self::parse_inlines(node, TextFormat::default(), &InlinesMode::AllowImages)?;
        let elements = inlines
            .into_iter()
            .map(|inline| match inline {
                Inline::Text(text) => ParagraphElement::Text(text),
                Inline::Image(url) => ParagraphElement::Image { url },
            })
            .collect();
        Ok(MarkdownElement::Paragraph(elements))
    }

    fn parse_text(node: &'a AstNode<'a>) -> ParseResult<Text> {
        let inlines = Self::parse_inlines(node, TextFormat::default(), &InlinesMode::DisallowImages)?;
        let chunks = inlines
            .into_iter()
            .flat_map(|inline| {
                let Inline::Text(text) = inline else { panic!("got non-text inline") };
                text.chunks.into_iter()
            })
            .collect();
        Ok(Text { chunks })
    }

    fn parse_inlines(node: &'a AstNode<'a>, format: TextFormat, mode: &InlinesMode) -> ParseResult<Vec<Inline>> {
        let mut inlines = Vec::new();
        let mut chunks = Vec::new();
        for node in node.children() {
            let value = &node.data.borrow().value;
            match value {
                NodeValue::Image(img) => {
                    if !chunks.is_empty() {
                        inlines.push(Inline::Text(Text { chunks: mem::take(&mut chunks) }));
                    }
                    inlines.push(Inline::Image(img.url.clone()));
                }
                _ => Self::collect_text_chunks(node, format.clone(), &mut chunks)?,
            };
        }
        if !chunks.is_empty() {
            inlines.push(Inline::Text(Text { chunks }));
        }
        let any_images = inlines.iter().any(|inline| matches!(inline, Inline::Image(_)));
        if matches!(mode, InlinesMode::DisallowImages) && any_images {
            return Err(ParseError::UnsupportedStructure { container: "text", element: "image" });
        }
        Ok(inlines)
    }

    fn collect_child_text_chunks(
        node: &'a AstNode<'a>,
        format: TextFormat,
        chunks: &mut Vec<TextChunk>,
    ) -> ParseResult<()> {
        for node in node.children() {
            Self::collect_text_chunks(node, format.clone(), chunks)?;
        }
        Ok(())
    }

    fn collect_text_chunks(node: &'a AstNode<'a>, format: TextFormat, chunks: &mut Vec<TextChunk>) -> ParseResult<()> {
        let value = &node.data.borrow().value;
        match value {
            NodeValue::Text(text) => {
                chunks.push(TextChunk::Formatted(FormattedText::formatted(text.clone(), format.clone())));
            }
            NodeValue::Code(code) => {
                chunks.push(TextChunk::Formatted(FormattedText::formatted(
                    code.literal.clone(),
                    TextFormat::default().code(),
                )));
            }
            NodeValue::Strong => Self::collect_child_text_chunks(node, format.clone().bold(), chunks)?,
            NodeValue::Emph => Self::collect_child_text_chunks(node, format.clone().italics(), chunks)?,
            NodeValue::Strikethrough => Self::collect_child_text_chunks(node, format.clone().strikethrough(), chunks)?,
            NodeValue::SoftBreak | NodeValue::LineBreak => chunks.push(TextChunk::LineBreak),
            other => return Err(ParseError::UnsupportedStructure { container: "text", element: other.identifier() }),
        };
        Ok(())
    }

    fn parse_list(root: &'a AstNode<'a>, depth: u8) -> ParseResult<Vec<ListItem>> {
        let mut elements = Vec::new();
        for (index, node) in root.children().enumerate() {
            let number = (index + 1) as u16;
            let value = &node.data.borrow().value;
            match value {
                NodeValue::Item(item) => {
                    elements.extend(Self::parse_list_item(item, node, depth, number)?);
                }
                other => {
                    return Err(ParseError::UnsupportedStructure { container: "list", element: other.identifier() });
                }
            };
        }
        Ok(elements)
    }

    fn parse_list_item(item: &NodeList, root: &'a AstNode<'a>, depth: u8, number: u16) -> ParseResult<Vec<ListItem>> {
        let item_type = match (item.list_type, item.delimiter) {
            (ListType::Bullet, _) => ListItemType::Unordered,
            (ListType::Ordered, ListDelimType::Paren) => ListItemType::OrderedParens(number),
            (ListType::Ordered, ListDelimType::Period) => ListItemType::OrderedPeriod(number),
        };
        let mut elements = Vec::new();
        for node in root.children() {
            let value = &node.data.borrow().value;
            match value {
                NodeValue::Paragraph => {
                    let contents = Self::parse_text(node)?;
                    elements.push(ListItem { contents, depth, item_type: item_type.clone() });
                }
                NodeValue::List(_) => {
                    elements.extend(Self::parse_list(node, depth + 1)?);
                }
                other => {
                    return Err(ParseError::UnsupportedStructure { container: "list", element: other.identifier() });
                }
            }
        }
        Ok(elements)
    }

    fn parse_table(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let mut header = TableRow(Vec::new());
        let mut rows = Vec::new();
        for node in node.children() {
            let value = &node.data.borrow().value;
            let NodeValue::TableRow(_) = value else {
                return Err(ParseError::UnsupportedStructure { container: "table", element: value.identifier() });
            };
            let row = Self::parse_table_row(node)?;
            if header.0.is_empty() {
                header = row;
            } else {
                rows.push(row)
            }
        }
        Ok(MarkdownElement::Table(Table { header, rows }))
    }

    fn parse_table_row(node: &'a AstNode<'a>) -> ParseResult<TableRow> {
        let mut cells = Vec::new();
        for node in node.children() {
            let value = &node.data.borrow().value;
            let NodeValue::TableCell = value else {
                return Err(ParseError::UnsupportedStructure { container: "table", element: value.identifier() });
            };
            let text = Self::parse_text(node)?;
            cells.push(text);
        }
        Ok(TableRow(cells))
    }
}

enum InlinesMode {
    AllowImages,
    DisallowImages,
}

enum Inline {
    Text(Text),
    Image(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("unsupported element: {0}")]
    UnsupportedElement(&'static str),

    #[error("unsupported structure in {container}: {element}")]
    UnsupportedStructure { container: &'static str, element: &'static str },

    #[error("only fenced code blocks are supported")]
    UnfencedCodeBlock,

    #[error("invalid metadata: {0}")]
    InvalidMetadata(String),
}

trait Identifier {
    fn identifier(&self) -> &'static str;
}

impl Identifier for NodeValue {
    fn identifier(&self) -> &'static str {
        match self {
            NodeValue::Document => "document",
            NodeValue::FrontMatter(_) => "front matter",
            NodeValue::BlockQuote => "block quote",
            NodeValue::List(_) => "list",
            NodeValue::Item(_) => "item",
            NodeValue::DescriptionList => "description list",
            NodeValue::DescriptionItem(_) => "description item",
            NodeValue::DescriptionTerm => "description term",
            NodeValue::DescriptionDetails => "description details",
            NodeValue::CodeBlock(_) => "code block",
            NodeValue::HtmlBlock(_) => "html block",
            NodeValue::Paragraph => "paragraph",
            NodeValue::Heading(_) => "heading",
            NodeValue::ThematicBreak => "thematic break",
            NodeValue::FootnoteDefinition(_) => "footnote definition",
            NodeValue::Table(_) => "table",
            NodeValue::TableRow(_) => "table row",
            NodeValue::TableCell => "table cell",
            NodeValue::Text(_) => "text",
            NodeValue::TaskItem(_) => "task item",
            NodeValue::SoftBreak => "soft break",
            NodeValue::LineBreak => "line break",
            NodeValue::Code(_) => "code",
            NodeValue::HtmlInline(_) => "inline html",
            NodeValue::Emph => "emph",
            NodeValue::Strong => "strong",
            NodeValue::Strikethrough => "strikethrough",
            NodeValue::Superscript => "superscript",
            NodeValue::Link(_) => "link",
            NodeValue::Image(_) => "image",
            NodeValue::FootnoteReference(_) => "footnote reference",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse_single(input: &str) -> MarkdownElement {
        let arena = Arena::new();
        let root = parse_document(&arena, input, &ParserOptions::default().0);
        assert_eq!(root.children().count(), 1, "expected a single child");

        let result = MarkdownParser::parse_element(root.first_child().unwrap()).expect("parsing failed");
        result
    }

    #[test]
    fn slide_metadata() {
        let parsed = parse_single(
            r"---
title: hello world
sub_title: hola
author: epic potato
---
",
        );
        let MarkdownElement::PresentationMetadata(inner) = parsed else {
            panic!("not a presentation title: {parsed:?}")
        };
        assert_eq!(inner.title, "hello world");
        assert_eq!(inner.sub_title, Some("hola".into()));
        assert_eq!(inner.author, Some("epic potato".into()));
    }

    #[test]
    fn paragraph() {
        let parsed = parse_single("some **bold text**, _italics_, *italics*, **nested _italics_**, ~strikethrough~");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks: Vec<_> = [
            FormattedText::plain("some "),
            FormattedText::formatted("bold text", TextFormat::default().bold()),
            FormattedText::plain(", "),
            FormattedText::formatted("italics", TextFormat::default().italics()),
            FormattedText::plain(", "),
            FormattedText::formatted("italics", TextFormat::default().italics()),
            FormattedText::plain(", "),
            FormattedText::formatted("nested ", TextFormat::default().bold()),
            FormattedText::formatted("italics", TextFormat::default().italics().bold()),
            FormattedText::plain(", "),
            FormattedText::formatted("strikethrough", TextFormat::default().strikethrough()),
        ]
        .into_iter()
        .map(TextChunk::Formatted)
        .collect();

        let expected_elements = &[ParagraphElement::Text(Text { chunks: expected_chunks })];
        assert_eq!(elements, expected_elements);
    }

    #[test]
    fn image() {
        let parsed = parse_single("![](potato.png)");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        assert_eq!(elements.len(), 1);
        let ParagraphElement::Image { url } = &elements[0] else { panic!("not an image") };
        assert_eq!(url, "potato.png");
    }

    #[test]
    fn slide_title() {
        let parsed = parse_single(
            r"
Title
===
",
        );
        let MarkdownElement::SlideTitle { text } = parsed else { panic!("not a slide title: {parsed:?}") };
        let expected_chunks = [TextChunk::Formatted(FormattedText::plain("Title"))];
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn heading() {
        let parsed = parse_single("# Title **with bold**");
        let MarkdownElement::Heading { text, level } = parsed else { panic!("not a heading: {parsed:?}") };
        let expected_chunks: Vec<_> =
            [FormattedText::plain("Title "), FormattedText::formatted("with bold", TextFormat::default().bold())]
                .into_iter()
                .map(TextChunk::Formatted)
                .collect();

        assert_eq!(level, 1);
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn unordered_list() {
        let parsed = parse_single(
            r"
 * One
    * Sub1
    * Sub2
 * Two
 * Three",
        );
        let MarkdownElement::List(items) = parsed else { panic!("not a list: {parsed:?}") };
        let mut items = items.into_iter();
        let mut next = || items.next().expect("list ended prematurely");
        assert_eq!(next().depth, 0);
        assert_eq!(next().depth, 1);
        assert_eq!(next().depth, 1);
        assert_eq!(next().depth, 0);
        assert_eq!(next().depth, 0);
    }

    #[test]
    fn line_break() {
        let parsed = parse_single(
            r"
some text
with line breaks",
        );
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a line break: {parsed:?}") };
        assert_eq!(elements.len(), 1);

        let expected_chunks = &[
            TextChunk::Formatted(FormattedText::plain("some text")),
            TextChunk::LineBreak,
            TextChunk::Formatted(FormattedText::plain("with line breaks")),
        ];
        let ParagraphElement::Text(text) = &elements[0] else { panic!("non-text in paragraph") };
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn code_block() {
        let parsed = parse_single(
            r"
```rust
let q = 42;
````
",
        );
        let MarkdownElement::Code(code) = parsed else { panic!("not a code block: {parsed:?}") };
        assert_eq!(code.language, CodeLanguage::Rust);
        assert_eq!(code.contents, "let q = 42;\n");
    }

    #[test]
    fn inline_code() {
        let parsed = parse_single("some `inline code`");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks = &[
            TextChunk::Formatted(FormattedText::plain("some ")),
            TextChunk::Formatted(FormattedText::formatted("inline code", TextFormat::default().code())),
        ];
        assert_eq!(elements.len(), 1);

        let ParagraphElement::Text(text) = &elements[0] else { panic!("non-text in paragraph") };
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn table() {
        let parsed = parse_single(
            r"
| Name | Taste |
| ------ | ------ |
| Potato | Great |
| Carrot | Yuck |
",
        );
        let MarkdownElement::Table(Table { header, rows }) = parsed else { panic!("not a table: {parsed:?}") };
        assert_eq!(header.0.len(), 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.len(), 2);
        assert_eq!(rows[1].0.len(), 2);
    }
}
