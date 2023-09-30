use crate::{
    markdown::elements::{
        Code, CodeLanguage, ListItem, ListItemType, MarkdownElement, ParagraphElement, StyledText, Table, TableRow,
        Text,
    },
    style::TextStyle,
};
use comrak::{
    format_commonmark,
    nodes::{
        AstNode, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeHtmlBlock, NodeList, NodeValue, Sourcepos,
    },
    parse_document, Arena, ComrakOptions, ListStyleType,
};
use std::{
    fmt::{self, Debug, Display},
    io::BufWriter,
    mem,
};

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

    pub fn parse(&self, contents: &str) -> ParseResult<Vec<MarkdownElement>> {
        let node = parse_document(self.arena, contents, &self.options);
        let mut elements = Vec::new();
        for node in node.children() {
            let element = Self::parse_node(node)?;
            elements.extend(element);
        }
        Ok(elements)
    }

    fn parse_node(node: &'a AstNode<'a>) -> ParseResult<Vec<MarkdownElement>> {
        let data = node.data.borrow();
        let element = match &data.value {
            // Paragraphs are the only ones that can actually yield more than one.
            NodeValue::Paragraph => return Self::parse_paragraph(node),
            NodeValue::FrontMatter(contents) => Self::parse_front_matter(contents)?,
            NodeValue::Heading(heading) => Self::parse_heading(heading, node)?,
            NodeValue::List(list) => {
                let items = Self::parse_list(node, list.marker_offset as u8 / 2)?;
                MarkdownElement::List(items)
            }
            NodeValue::Table(_) => Self::parse_table(node)?,
            NodeValue::CodeBlock(block) => Self::parse_code_block(block, data.sourcepos)?,
            NodeValue::ThematicBreak => MarkdownElement::ThematicBreak,
            NodeValue::HtmlBlock(block) => Self::parse_html_block(block, data.sourcepos)?,
            NodeValue::BlockQuote => Self::parse_block_quote(node)?,
            other => return Err(ParseErrorKind::UnsupportedElement(other.identifier()).with_sourcepos(data.sourcepos)),
        };
        Ok(vec![element])
    }

    fn parse_front_matter(contents: &str) -> ParseResult<MarkdownElement> {
        // Remote leading and trailing delimiters before parsing. This is quite poopy but hey, it
        // works.
        let contents = contents.strip_prefix("---\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n\n").unwrap_or(contents);
        Ok(MarkdownElement::FrontMatter(contents.into()))
    }

    fn parse_html_block(block: &NodeHtmlBlock, sourcepos: Sourcepos) -> ParseResult<MarkdownElement> {
        let block = block.literal.trim();
        let start_tag = "<!--";
        let end_tag = "-->";
        if !block.starts_with(start_tag) || !block.ends_with(end_tag) {
            return Err(ParseErrorKind::UnsupportedElement("html block").with_sourcepos(sourcepos));
        }
        let block = &block[start_tag.len()..];
        let block = &block[0..block.len() - end_tag.len()];
        let block = block.trim();
        Ok(MarkdownElement::Comment(block.into()))
    }

    fn parse_block_quote(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let mut buffer = BufWriter::new(Vec::new());
        let mut options = ParserOptions::default().0;
        options.render.list_style = ListStyleType::Star;
        format_commonmark(node, &options, &mut buffer)
            .map_err(|e| ParseErrorKind::Internal(e.to_string()).with_sourcepos(node.data.borrow().sourcepos))?;

        let buffer = buffer.into_inner().expect("unwrapping writer failed");
        let mut lines = Vec::new();
        for line in String::from_utf8_lossy(&buffer).lines() {
            let line = match line.find('>') {
                Some(index) => line[index + 1..].trim(),
                None => line,
            };
            lines.push(line.to_string());
        }
        Ok(MarkdownElement::BlockQuote(lines))
    }

    fn parse_code_block(block: &NodeCodeBlock, sourcepos: Sourcepos) -> ParseResult<MarkdownElement> {
        if !block.fenced {
            return Err(ParseErrorKind::UnfencedCodeBlock.with_sourcepos(sourcepos));
        }
        use CodeLanguage::*;
        let language = match block.info.as_str() {
            "asp" => Asp,
            "bash" => Bash,
            "c" => C,
            "csharp" => CSharp,
            "clojure" => Clojure,
            "cpp" | "c++" => Cpp,
            "css" => Css,
            "d" => DLang,
            "erlang" => Erlang,
            "go" => Go,
            "haskell" => Haskell,
            "html" => Html,
            "java" => Java,
            "javascript" | "js" => JavaScript,
            "json" => Json,
            "latex" => Latex,
            "lua" => Lua,
            "make" => Makefile,
            "markdown" => Markdown,
            "ocaml" => OCaml,
            "perl" => Perl,
            "php" => Php,
            "python" => Python,
            "r" => R,
            "rust" => Rust,
            "scala" => Scala,
            "shell" | "sh" | "zsh" | "fish" => Shell,
            "sql" => Sql,
            "typescript" | "ts" => TypeScript,
            "xml" => Xml,
            "yaml" => Yaml,
            _ => Unknown,
        };
        let code = Code { contents: block.literal.clone(), language };
        Ok(MarkdownElement::Code(code))
    }

    fn parse_heading(heading: &NodeHeading, node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let text = Self::parse_text(node)?;
        if heading.setext {
            Ok(MarkdownElement::SetexHeading { text })
        } else {
            Ok(MarkdownElement::Heading { text, level: heading.level })
        }
    }

    fn parse_paragraph(node: &'a AstNode<'a>) -> ParseResult<Vec<MarkdownElement>> {
        let mut elements = Vec::new();
        let inlines = InlinesParser::new(InlinesMode::AllowImages).parse(node)?;
        let mut paragraph_elements = Vec::new();
        for inline in inlines {
            match inline {
                Inline::Text(text) => paragraph_elements.push(ParagraphElement::Text(text)),
                Inline::LineBreak => paragraph_elements.push(ParagraphElement::LineBreak),
                Inline::Image(path) => {
                    if !paragraph_elements.is_empty() {
                        elements.push(MarkdownElement::Paragraph(mem::take(&mut paragraph_elements)));
                    }
                    elements.push(MarkdownElement::Image(path));
                }
            }
        }
        if !paragraph_elements.is_empty() {
            elements.push(MarkdownElement::Paragraph(mem::take(&mut paragraph_elements)));
        }
        Ok(elements)
    }

    fn parse_text(node: &'a AstNode<'a>) -> ParseResult<Text> {
        let inlines = InlinesParser::new(InlinesMode::DisallowImages).parse(node)?;
        let chunks = inlines
            .into_iter()
            .flat_map(|inline| {
                let Inline::Text(text) = inline else { panic!("got non-text inline") };
                text.chunks.into_iter()
            })
            .collect();
        Ok(Text { chunks })
    }

    fn parse_list(root: &'a AstNode<'a>, depth: u8) -> ParseResult<Vec<ListItem>> {
        let mut elements = Vec::new();
        for (index, node) in root.children().enumerate() {
            let number = (index + 1) as u16;
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Item(item) => {
                    elements.extend(Self::parse_list_item(item, node, depth, number)?);
                }
                other => {
                    return Err(ParseErrorKind::UnsupportedStructure {
                        container: "list",
                        element: other.identifier(),
                    }
                    .with_sourcepos(data.sourcepos));
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
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Paragraph => {
                    let contents = Self::parse_text(node)?;
                    elements.push(ListItem { contents, depth, item_type: item_type.clone() });
                }
                NodeValue::List(_) => {
                    elements.extend(Self::parse_list(node, depth + 1)?);
                }
                other => {
                    return Err(ParseErrorKind::UnsupportedStructure {
                        container: "list",
                        element: other.identifier(),
                    }
                    .with_sourcepos(data.sourcepos));
                }
            }
        }
        Ok(elements)
    }

    fn parse_table(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let mut header = TableRow(Vec::new());
        let mut rows = Vec::new();
        for node in node.children() {
            let data = node.data.borrow();
            let NodeValue::TableRow(_) = &data.value else {
                return Err(ParseErrorKind::UnsupportedStructure {
                    container: "table",
                    element: data.value.identifier(),
                }
                .with_sourcepos(data.sourcepos));
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
            let data = node.data.borrow();
            let NodeValue::TableCell = &data.value else {
                return Err(ParseErrorKind::UnsupportedStructure {
                    container: "table",
                    element: data.value.identifier(),
                }
                .with_sourcepos(data.sourcepos));
            };
            let text = Self::parse_text(node)?;
            cells.push(text);
        }
        Ok(TableRow(cells))
    }
}

struct InlinesParser {
    mode: InlinesMode,
    inlines: Vec<Inline>,
    text_chunks: Vec<StyledText>,
}

impl InlinesParser {
    fn new(mode: InlinesMode) -> Self {
        Self { mode, inlines: Default::default(), text_chunks: Default::default() }
    }

    fn parse<'a>(mut self, node: &'a AstNode<'a>) -> ParseResult<Vec<Inline>> {
        for node in node.children() {
            let value = &node.data.borrow().value;
            match value {
                NodeValue::Image(img) => {
                    self.store_pending_text();
                    self.inlines.push(Inline::Image(img.url.clone()));
                }
                _ => self.collect_text_chunks(node, TextStyle::default())?,
            };
        }
        self.store_pending_text();

        let any_images = self.inlines.iter().any(|inline| matches!(inline, Inline::Image(_)));
        if matches!(self.mode, InlinesMode::DisallowImages) && any_images {
            let sourcepos = node.data.borrow().sourcepos;
            return Err(
                ParseErrorKind::UnsupportedStructure { container: "text", element: "image" }.with_sourcepos(sourcepos)
            );
        }
        Ok(self.inlines)
    }

    fn store_pending_text(&mut self) {
        let chunks = mem::take(&mut self.text_chunks);
        if !chunks.is_empty() {
            self.inlines.push(Inline::Text(Text { chunks }));
        }
    }

    fn collect_child_text_chunks<'a>(&mut self, node: &'a AstNode<'a>, style: TextStyle) -> ParseResult<()> {
        for node in node.children() {
            self.collect_text_chunks(node, style.clone())?;
        }
        Ok(())
    }

    fn collect_text_chunks<'a>(&mut self, node: &'a AstNode<'a>, style: TextStyle) -> ParseResult<()> {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Text(text) => {
                self.text_chunks.push(StyledText::styled(text.clone(), style.clone()));
            }
            NodeValue::Code(code) => {
                self.text_chunks.push(StyledText::styled(code.literal.clone(), TextStyle::default().code()));
            }
            NodeValue::Strong => self.collect_child_text_chunks(node, style.clone().bold())?,
            NodeValue::Emph => self.collect_child_text_chunks(node, style.clone().italics())?,
            NodeValue::Strikethrough => self.collect_child_text_chunks(node, style.clone().strikethrough())?,
            NodeValue::SoftBreak => self.text_chunks.push(StyledText::plain(" ")),
            NodeValue::Link(link) => {
                self.text_chunks.push(StyledText::styled(link.url.clone(), TextStyle::default().link()))
            }
            NodeValue::LineBreak => {
                self.store_pending_text();
                self.inlines.push(Inline::LineBreak);
            }
            other => {
                return Err(ParseErrorKind::UnsupportedStructure { container: "text", element: other.identifier() }
                    .with_sourcepos(data.sourcepos));
            }
        };
        Ok(())
    }
}

enum InlinesMode {
    AllowImages,
    DisallowImages,
}

enum Inline {
    Text(Text),
    Image(String),
    LineBreak,
}

#[derive(thiserror::Error, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub sourcepos: Sourcepos,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.sourcepos.start.line, self.sourcepos.start.column, self.kind)
    }
}

impl ParseError {
    fn new(kind: ParseErrorKind, sourcepos: Sourcepos) -> Self {
        Self { kind, sourcepos }
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    UnsupportedElement(&'static str),
    UnsupportedStructure { container: &'static str, element: &'static str },
    UnfencedCodeBlock,
    Internal(String),
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedElement(element) => write!(f, "unsupported element: {element}"),
            Self::UnsupportedStructure { container, element } => {
                write!(f, "unsupported structure in {container}: {element}")
            }
            Self::UnfencedCodeBlock => write!(f, "only fenced code blocks are supported"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
        }
    }
}

impl ParseErrorKind {
    fn with_sourcepos(self, sourcepos: Sourcepos) -> ParseError {
        ParseError::new(self, sourcepos)
    }
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
        let result = MarkdownParser::new(&arena).parse(input).expect("parsing failed");
        assert_eq!(result.len(), 1, "more than one element: {result:?}");
        result.into_iter().next().unwrap()
    }

    fn parse_all(input: &str) -> Vec<MarkdownElement> {
        let arena = Arena::new();
        let result = MarkdownParser::new(&arena).parse(input).expect("parsing failed");
        result
    }

    #[test]
    fn slide_metadata() {
        let parsed = parse_single(
            r"---
beep
boop
---
",
        );
        let MarkdownElement::FrontMatter(contents) = parsed else { panic!("not a front matter: {parsed:?}") };
        assert_eq!(contents, "beep\nboop\n");
    }

    #[test]
    fn paragraph() {
        let parsed = parse_single("some **bold text**, _italics_, *italics*, **nested _italics_**, ~strikethrough~");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks = vec![
            StyledText::plain("some "),
            StyledText::styled("bold text", TextStyle::default().bold()),
            StyledText::plain(", "),
            StyledText::styled("italics", TextStyle::default().italics()),
            StyledText::plain(", "),
            StyledText::styled("italics", TextStyle::default().italics()),
            StyledText::plain(", "),
            StyledText::styled("nested ", TextStyle::default().bold()),
            StyledText::styled("italics", TextStyle::default().italics().bold()),
            StyledText::plain(", "),
            StyledText::styled("strikethrough", TextStyle::default().strikethrough()),
        ];

        let expected_elements = &[ParagraphElement::Text(Text { chunks: expected_chunks })];
        assert_eq!(elements, expected_elements);
    }

    #[test]
    fn link() {
        let parsed = parse_single("my [website](https://example.com)");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks =
            vec![StyledText::plain("my "), StyledText::styled("https://example.com", TextStyle::default().link())];

        let expected_elements = &[ParagraphElement::Text(Text { chunks: expected_chunks })];
        assert_eq!(elements, expected_elements);
    }

    #[test]
    fn image() {
        let parsed = parse_single("![](potato.png)");
        let MarkdownElement::Image(path) = parsed else { panic!("not an image: {parsed:?}") };
        assert_eq!(path, "potato.png");
    }

    #[test]
    fn image_within_text() {
        let parsed = parse_all(
            r"
picture of potato: ![](potato.png)
",
        );
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn setex_heading() {
        let parsed = parse_single(
            r"
Title
===
",
        );
        let MarkdownElement::SetexHeading { text } = parsed else { panic!("not a slide title: {parsed:?}") };
        let expected_chunks = [StyledText::plain("Title")];
        assert_eq!(text.chunks, expected_chunks);
    }

    #[test]
    fn heading() {
        let parsed = parse_single("# Title **with bold**");
        let MarkdownElement::Heading { text, level } = parsed else { panic!("not a heading: {parsed:?}") };
        let expected_chunks =
            vec![StyledText::plain("Title "), StyledText::styled("with bold", TextStyle::default().bold())];

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
    fn line_breaks() {
        let parsed = parse_all(
            r"
some text
with line breaks  
a hard break

another",
        );
        // note that "with line breaks" also has a hard break ("  ") at the end, hence the 3.
        assert_eq!(parsed.len(), 2);

        let MarkdownElement::Paragraph(elements) = &parsed[0] else { panic!("not a line break: {parsed:?}") };
        assert_eq!(elements.len(), 3);

        let expected_chunks =
            &[StyledText::plain("some text"), StyledText::plain(" "), StyledText::plain("with line breaks")];
        let ParagraphElement::Text(text) = &elements[0] else { panic!("non-text in paragraph") };
        assert_eq!(text.chunks, expected_chunks);
        assert!(matches!(&elements[1], ParagraphElement::LineBreak));
        assert!(matches!(&elements[2], ParagraphElement::Text(_)));
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
        let expected_chunks =
            &[StyledText::plain("some "), StyledText::styled("inline code", TextStyle::default().code())];
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

    #[test]
    fn comment() {
        let parsed = parse_single(
            r"
<!-- foo -->
",
        );
        let MarkdownElement::Comment(text) = parsed else { panic!("not a comment: {parsed:?}") };
        assert_eq!(text, "foo");
    }

    #[test]
    fn list_comment_in_between() {
        let parsed = parse_all(
            r"
* A
<!-- foo -->
  * B
",
        );
        assert_eq!(parsed.len(), 3);
        let MarkdownElement::List(items) = &parsed[2] else { panic!("not a list item: {parsed:?}") };
        assert_eq!(items[0].depth, 1);
    }

    #[test]
    fn block_quote() {
        let parsed = parse_single(
            r"
> bar
> foo
> 
> * a
> * b
",
        );
        let MarkdownElement::BlockQuote(lines) = parsed else { panic!("not a block quote: {parsed:?}") };
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "bar");
        assert_eq!(lines[1], "foo");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "* a");
        assert_eq!(lines[4], "* b");
    }

    #[test]
    fn thematic_break() {
        let parsed = parse_all(
            r"
hello

---

bye
",
        );
        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[1], MarkdownElement::ThematicBreak));
    }
}
