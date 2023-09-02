use crate::{
    elements::{Code, CodeLanguage, Element, FormattedText, ListItem, ListItemType, Text, TextChunk, TextFormat},
    presentation::Slide,
};
use comrak::{
    nodes::{AstNode, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeList, NodeValue},
    parse_document, Arena, ComrakOptions,
};
use std::mem;

type ParseResult<T> = Result<T, ParseError>;

pub struct SlideParser<'a> {
    arena: &'a Arena<AstNode<'a>>,
    options: ComrakOptions,
}

impl<'a> SlideParser<'a> {
    pub fn new(arena: &'a Arena<AstNode<'a>>, options: ComrakOptions) -> Self {
        Self { arena, options }
    }

    pub fn parse(&self, document: &str) -> ParseResult<Vec<Slide>> {
        let root = parse_document(self.arena, document, &self.options);
        let mut slides = Vec::new();
        let mut slide_elements = Vec::new();
        for node in root.children() {
            let value = &node.data.borrow().value;
            match value {
                NodeValue::ThematicBreak => {
                    let slide = Slide::new(mem::take(&mut slide_elements));
                    slides.push(slide);
                    continue;
                }
                _ => {
                    slide_elements.push(Self::parse_element(node)?);
                }
            };
        }
        if !slide_elements.is_empty() {
            slides.push(Slide::new(slide_elements));
        }
        Ok(slides)
    }

    fn parse_element(node: &'a AstNode<'a>) -> ParseResult<Element> {
        let value = &node.data.borrow().value;
        match value {
            NodeValue::Heading(heading) => Self::parse_heading(heading, node),
            NodeValue::Paragraph => Self::parse_paragraph(node),
            NodeValue::List(_) => {
                let items = Self::parse_list(node, 0)?;
                Ok(Element::List(items))
            }
            NodeValue::CodeBlock(block) => Self::parse_code_block(block),
            other => Err(ParseError::UnsupportedElement(other.identifier())),
        }
    }

    fn parse_code_block(block: &NodeCodeBlock) -> ParseResult<Element> {
        if !block.fenced {
            return Err(ParseError::UnfencedCodeBlock);
        }
        // TODO less naive pls
        let language = if block.info.contains("rust") { CodeLanguage::Rust } else { CodeLanguage::Other };
        let code = Code { contents: block.literal.clone(), language };
        Ok(Element::Code(code))
    }

    fn parse_heading(heading: &NodeHeading, node: &'a AstNode<'a>) -> ParseResult<Element> {
        let text = Self::parse_text(node)?;
        if heading.setext {
            Ok(Element::SlideTitle { text })
        } else {
            Ok(Element::Heading { text, level: heading.level })
        }
    }

    fn parse_paragraph(node: &'a AstNode<'a>) -> ParseResult<Element> {
        let text = Self::parse_text(node)?;
        let element = Element::Paragraph(text);
        Ok(element)
    }

    fn parse_text(root: &'a AstNode<'a>) -> ParseResult<Text> {
        let chunks = Self::parse_text_chunks(root, TextFormat::default())?;
        Ok(Text { chunks })
    }

    fn parse_text_chunks(root: &'a AstNode<'a>, format: TextFormat) -> ParseResult<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        for node in root.children() {
            let value = &node.data.borrow().value;
            match value {
                NodeValue::Text(text) => {
                    chunks.push(TextChunk::Formatted(FormattedText::formatted(text.clone(), format.clone())));
                }
                NodeValue::Strong => chunks.extend(Self::parse_text_chunks(node, format.clone().add_bold())?),
                NodeValue::Emph => chunks.extend(Self::parse_text_chunks(node, format.clone().add_italics())?),
                NodeValue::Image(img) => {
                    chunks.push(TextChunk::Image { title: img.title.clone(), url: img.url.clone() });
                }
                NodeValue::SoftBreak | NodeValue::LineBreak => chunks.push(TextChunk::LineBreak),
                other => {
                    return Err(ParseError::UnsupportedStructure { container: "text", element: other.identifier() })
                }
            };
        }
        Ok(chunks)
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
                    return Err(ParseError::UnsupportedStructure { container: "list", element: other.identifier() })
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
                    return Err(ParseError::UnsupportedStructure { container: "list", element: other.identifier() })
                }
            }
        }
        Ok(elements)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("unsupported element: {0}")]
    UnsupportedElement(&'static str),

    #[error("unsupported structure in {container}: {element}")]
    UnsupportedStructure { container: &'static str, element: &'static str },

    #[error("only fenced code blocks are supported")]
    UnfencedCodeBlock,
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

    fn parse_single(input: &str) -> Element {
        let arena = Arena::new();
        let root = parse_document(&arena, input, &ComrakOptions::default());
        assert_eq!(root.children().count(), 1, "expected a single child");

        let result = SlideParser::parse_element(root.first_child().unwrap()).expect("parsing failed");
        result
    }

    fn parse_slides(input: &str) -> Vec<Slide> {
        let arena = Arena::new();
        let parser = SlideParser::new(&arena, ComrakOptions::default());
        parser.parse(input).expect("parsing failed")
    }

    #[test]
    fn paragraph() {
        let parsed = parse_single("some **bold text**, _italics_, *italics*, **nested _italics_**");
        let Element::Paragraph(text) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks: Vec<_> = [
            FormattedText::plain("some "),
            FormattedText::formatted("bold text", TextFormat::default().add_bold()),
            FormattedText::plain(", "),
            FormattedText::formatted("italics", TextFormat::default().add_italics()),
            FormattedText::plain(", "),
            FormattedText::formatted("italics", TextFormat::default().add_italics()),
            FormattedText::plain(", "),
            FormattedText::formatted("nested ", TextFormat::default().add_bold()),
            FormattedText::formatted("italics", TextFormat::default().add_italics().add_bold()),
        ]
        .into_iter()
        .map(TextChunk::Formatted)
        .collect();
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn image() {
        let parsed = parse_single("![](potato.png \"hi\")");
        let Element::Paragraph(text) = parsed else { panic!("not a paragraph: {parsed:?}") };
        assert_eq!(text.chunks.len(), 1);
        let TextChunk::Image{title, url} = &text.chunks[0] else { panic!("not an image") };
        assert_eq!(title, "hi");
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
        let Element::SlideTitle { text} = parsed else { panic!("not a slide title: {parsed:?}") };
        let expected_chunks = [TextChunk::Formatted(FormattedText::plain("Title"))];
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn heading() {
        let parsed = parse_single("# Title **with bold**");
        let Element::Heading { text, level } = parsed else { panic!("not a heading: {parsed:?}") };
        let expected_chunks: Vec<_> =
            [FormattedText::plain("Title "), FormattedText::formatted("with bold", TextFormat::default().add_bold())]
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
        let Element::List(items) = parsed else { panic!("not a list: {parsed:?}") };
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
        let Element::Paragraph(text) = parsed else { panic!("not a line break: {parsed:?}") };
        let expected_chunks = &[
            TextChunk::Formatted(FormattedText::plain("some text")),
            TextChunk::LineBreak,
            TextChunk::Formatted(FormattedText::plain("with line breaks")),
        ];

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
        let Element::Code(code) = parsed else { panic!("not a code block: {parsed:?}") };
        assert_eq!(code.language, CodeLanguage::Rust);
        assert_eq!(code.contents, "let q = 42;\n");
    }

    #[test]
    fn slide_splitting() {
        let slides = parse_slides(
            "First

---
Second

***
Third
",
        );
        assert_eq!(slides.len(), 3);

        assert_eq!(slides[0].elements.len(), 1);
        assert_eq!(slides[1].elements.len(), 1);
        assert_eq!(slides[2].elements.len(), 1);

        let expected = ["First", "Second", "Third"];
        for (slide, expected) in slides.into_iter().zip(expected) {
            let Element::Paragraph(text) = &slide.elements[0] else { panic!("no text") };
            let chunks = [TextChunk::Formatted(FormattedText::plain(expected))];
            assert_eq!(text.chunks, chunks);
        }
    }
}
