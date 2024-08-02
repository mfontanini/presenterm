use super::{code::CodeBlockParseError, elements::SourcePosition};
use crate::{
    markdown::{
        code::CodeBlockParser,
        elements::{ListItem, ListItemType, MarkdownElement, ParagraphElement, Table, TableRow, Text, TextBlock},
    },
    style::TextStyle,
    Resources,
};
use comrak::{
    arena_tree::Node,
    format_commonmark,
    nodes::{
        Ast, AstNode, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeHtmlBlock, NodeList, NodeValue,
        Sourcepos,
    },
    parse_document, Arena, ComrakOptions, ListStyleType,
};
use std::{
    cell::RefCell,
    fmt::{self, Debug, Display},
    io::BufWriter,
    mem,
};

/// The result of parsing a markdown file.
pub(crate) type ParseResult<T> = Result<T, ParseError>;

struct ParserOptions(ComrakOptions<'static>);

impl Default for ParserOptions {
    fn default() -> Self {
        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".into());
        options.extension.table = true;
        options.extension.strikethrough = true;
        options.extension.multiline_block_quotes = true;
        Self(options)
    }
}

/// A markdown parser.
///
/// This takes the contents of a markdown file and parses it into a list of [MarkdownElement].
pub struct MarkdownParser<'a> {
    arena: &'a Arena<AstNode<'a>>,
    options: ComrakOptions<'static>,
}

impl<'a> MarkdownParser<'a> {
    /// Construct a new markdown parser.
    pub fn new(arena: &'a Arena<AstNode<'a>>) -> Self {
        Self { arena, options: ParserOptions::default().0 }
    }

    /// Parse the contents of a markdown file.
    pub(crate) fn parse(&self, contents: &str, resources: &mut Resources) -> ParseResult<Vec<MarkdownElement>> {
        let node = parse_document(self.arena, contents, &self.options);
        let mut elements = Vec::new();
        let mut lines_offset = 0;
        for node in node.children() {
            let mut parsed_elements = self
                .parse_node(node, resources)
                .map_err(|e| ParseError::new(e.kind, e.sourcepos.offset_lines(lines_offset)))?;
            if let Some(MarkdownElement::FrontMatter(contents)) = parsed_elements.first() {
                lines_offset += contents.lines().count() + 2;
            }
            // comrak ignores the lines in the front matter so we need to offset this ourselves.
            Self::adjust_source_positions(parsed_elements.iter_mut(), lines_offset);
            elements.extend(parsed_elements);
        }
        Ok(elements)
    }

    fn adjust_source_positions<'b>(elements: impl Iterator<Item = &'b mut MarkdownElement>, lines_offset: usize) {
        for element in elements {
            let position = match element {
                MarkdownElement::FrontMatter(_)
                | MarkdownElement::SetexHeading { .. }
                | MarkdownElement::Heading { .. }
                | MarkdownElement::Paragraph(_)
                | MarkdownElement::Image { .. }
                | MarkdownElement::List(_)
                | MarkdownElement::Snippet(_)
                | MarkdownElement::Table(_)
                | MarkdownElement::ThematicBreak
                | MarkdownElement::BlockQuote(_) => continue,
                MarkdownElement::Comment { source_position, .. } => source_position,
            };
            *position = position.offset_lines(lines_offset);
        }
    }

    fn parse_node(&self, node: &'a AstNode<'a>, resources: &mut Resources) -> ParseResult<Vec<MarkdownElement>> {
        let data = node.data.borrow();
        let element = match &data.value {
            // Paragraphs are the only ones that can actually yield more than one.
            NodeValue::Paragraph => return self.parse_paragraph(node),
            NodeValue::FrontMatter(contents) => Self::parse_front_matter(contents)?,
            NodeValue::Heading(heading) => self.parse_heading(heading, node)?,
            NodeValue::List(list) => {
                let items = self.parse_list(node, list.marker_offset as u8 / 2)?;
                MarkdownElement::List(items)
            }
            NodeValue::Table(_) => self.parse_table(node)?,
            NodeValue::CodeBlock(block) => Self::parse_code_block(block, data.sourcepos, resources)?,
            NodeValue::ThematicBreak => MarkdownElement::ThematicBreak,
            NodeValue::HtmlBlock(block) => Self::parse_html_block(block, data.sourcepos)?,
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => Self::parse_block_quote(node)?,
            other => return Err(ParseErrorKind::UnsupportedElement(other.identifier()).with_sourcepos(data.sourcepos)),
        };
        Ok(vec![element])
    }

    fn parse_front_matter(contents: &str) -> ParseResult<MarkdownElement> {
        // Remote leading and trailing delimiters before parsing. This is quite poopy but hey, it
        // works.
        let contents = contents.strip_prefix("---\n").unwrap_or(contents);
        let contents = contents.strip_prefix("---\r\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\r\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\n\n").unwrap_or(contents);
        let contents = contents.strip_suffix("---\r\n\r\n").unwrap_or(contents);
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
        Ok(MarkdownElement::Comment { comment: block.into(), source_position: sourcepos.into() })
    }

    fn parse_block_quote(node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        // This renders the contents of this block quote AST as commonmark, given we otherwise
        // would need to either do this outselves or pull the raw block contents off of the
        // original raw string and that also isn't great.
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
            let mut line = line.to_string();
            // `format_commonmark` escapes these symbols so we un-escape them.
            for escape in &["\\*", "\\!", "\\[", "\\]", "\\#", "\\`", "\\<", "\\>"] {
                if line.contains(escape) {
                    line = line.replace(escape, &escape[1..]);
                }
            }
            lines.push(line);
        }
        Ok(MarkdownElement::BlockQuote(lines))
    }

    fn parse_code_block(
        block: &NodeCodeBlock,
        sourcepos: Sourcepos,
        resources: &mut Resources,
    ) -> ParseResult<MarkdownElement> {
        if !block.fenced {
            return Err(ParseErrorKind::UnfencedCodeBlock.with_sourcepos(sourcepos));
        }
        let code = CodeBlockParser::parse(block, resources)
            .map_err(|e| ParseErrorKind::InvalidCodeBlock(e).with_sourcepos(sourcepos))?;
        Ok(MarkdownElement::Snippet(code))
    }

    fn parse_heading(&self, heading: &NodeHeading, node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
        let text = self.parse_text(node)?;
        if heading.setext {
            Ok(MarkdownElement::SetexHeading { text })
        } else {
            Ok(MarkdownElement::Heading { text, level: heading.level })
        }
    }

    fn parse_paragraph(&self, node: &'a AstNode<'a>) -> ParseResult<Vec<MarkdownElement>> {
        let mut elements = Vec::new();
        let inlines = InlinesParser::new(self.arena).parse(node)?;
        let mut paragraph_elements = Vec::new();
        for inline in inlines {
            match inline {
                Inline::Text(text) => paragraph_elements.push(ParagraphElement::Text(text)),
                Inline::LineBreak => paragraph_elements.push(ParagraphElement::LineBreak),
                Inline::Image { path, title } => {
                    if !paragraph_elements.is_empty() {
                        elements.push(MarkdownElement::Paragraph(mem::take(&mut paragraph_elements)));
                    }
                    elements.push(MarkdownElement::Image {
                        path: path.into(),
                        title,
                        source_position: node.data.borrow().sourcepos.into(),
                    });
                }
            }
        }
        if !paragraph_elements.is_empty() {
            elements.push(MarkdownElement::Paragraph(mem::take(&mut paragraph_elements)));
        }
        Ok(elements)
    }

    fn parse_text(&self, node: &'a AstNode<'a>) -> ParseResult<TextBlock> {
        let inlines = InlinesParser::new(self.arena).parse(node)?;
        let mut chunks = Vec::new();
        for inline in inlines {
            match inline {
                Inline::Text(text) => chunks.extend(text.0),
                other => {
                    return Err(ParseErrorKind::UnsupportedStructure { container: "text", element: other.kind() }
                        .with_sourcepos(node.data.borrow().sourcepos));
                }
            };
        }
        Ok(TextBlock(chunks))
    }

    fn parse_list(&self, root: &'a AstNode<'a>, depth: u8) -> ParseResult<Vec<ListItem>> {
        let mut elements = Vec::new();
        for node in root.children() {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Item(item) => {
                    elements.extend(self.parse_list_item(item, node, depth)?);
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

    fn parse_list_item(&self, item: &NodeList, root: &'a AstNode<'a>, depth: u8) -> ParseResult<Vec<ListItem>> {
        let item_type = match (item.list_type, item.delimiter) {
            (ListType::Bullet, _) => ListItemType::Unordered,
            (ListType::Ordered, ListDelimType::Paren) => ListItemType::OrderedParens,
            (ListType::Ordered, ListDelimType::Period) => ListItemType::OrderedPeriod,
        };
        let mut elements = Vec::new();
        for node in root.children() {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Paragraph => {
                    let contents = self.parse_text(node)?;
                    elements.push(ListItem { contents, depth, item_type: item_type.clone() });
                }
                NodeValue::List(_) => {
                    elements.extend(self.parse_list(node, depth + 1)?);
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

    fn parse_table(&self, node: &'a AstNode<'a>) -> ParseResult<MarkdownElement> {
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
            let row = self.parse_table_row(node)?;
            if header.0.is_empty() {
                header = row;
            } else {
                rows.push(row)
            }
        }
        Ok(MarkdownElement::Table(Table { header, rows }))
    }

    fn parse_table_row(&self, node: &'a AstNode<'a>) -> ParseResult<TableRow> {
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
            let text = self.parse_text(node)?;
            cells.push(text);
        }
        Ok(TableRow(cells))
    }
}

struct InlinesParser<'a> {
    inlines: Vec<Inline>,
    pending_text: Vec<Text>,
    arena: &'a Arena<AstNode<'a>>,
}

impl<'a> InlinesParser<'a> {
    fn new(arena: &'a Arena<AstNode<'a>>) -> Self {
        Self { inlines: Vec::new(), pending_text: Vec::new(), arena }
    }

    fn parse(mut self, node: &'a AstNode<'a>) -> ParseResult<Vec<Inline>> {
        self.process_children(node, TextStyle::default())?;
        self.store_pending_text();
        Ok(self.inlines)
    }

    fn store_pending_text(&mut self) {
        let chunks = mem::take(&mut self.pending_text);
        if !chunks.is_empty() {
            self.inlines.push(Inline::Text(TextBlock(chunks)));
        }
    }

    fn process_node(&mut self, node: &'a AstNode<'a>, style: TextStyle) -> ParseResult<()> {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Text(text) => {
                self.pending_text.push(Text::new(text.clone(), style));
            }
            NodeValue::Code(code) => {
                self.pending_text.push(Text::new(code.literal.clone(), TextStyle::default().code()));
            }
            NodeValue::Strong => self.process_children(node, style.bold())?,
            NodeValue::Emph => self.process_children(node, style.italics())?,
            NodeValue::Strikethrough => self.process_children(node, style.strikethrough())?,
            NodeValue::SoftBreak => self.pending_text.push(Text::from(" ")),
            NodeValue::Link(link) => self.pending_text.push(Text::new(link.url.clone(), TextStyle::default().link())),
            NodeValue::LineBreak => {
                self.store_pending_text();
                self.inlines.push(Inline::LineBreak);
            }
            NodeValue::Image(link) => {
                self.store_pending_text();

                // The image "title" contains inlines so we create a dummy paragraph node that
                // contains it so we can flatten it back into text. We could walk the tree but this
                // is good enough.
                let mut buffer = Vec::new();
                let paragraph =
                    self.arena.alloc(Node::new(RefCell::new(Ast::new(NodeValue::Paragraph, data.sourcepos.start))));
                for child in node.children() {
                    paragraph.append(child);
                }
                format_commonmark(paragraph, &ParserOptions::default().0, &mut buffer)
                    .map_err(|e| ParseErrorKind::Internal(e.to_string()).with_sourcepos(data.sourcepos))?;

                let title = String::from_utf8_lossy(&buffer).trim_end().to_string();
                self.inlines.push(Inline::Image { path: link.url.clone(), title });
            }
            other => {
                return Err(ParseErrorKind::UnsupportedStructure { container: "text", element: other.identifier() }
                    .with_sourcepos(data.sourcepos));
            }
        };
        Ok(())
    }

    fn process_children(&mut self, node: &'a AstNode<'a>, style: TextStyle) -> ParseResult<()> {
        for node in node.children() {
            self.process_node(node, style)?;
        }
        Ok(())
    }
}

enum Inline {
    Text(TextBlock),
    Image { path: String, title: String },
    LineBreak,
}

impl Inline {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Image { .. } => "image",
            Self::LineBreak => "line break",
        }
    }
}

/// A parsing error.
#[derive(thiserror::Error, Debug)]
pub struct ParseError {
    /// The kind of error.
    pub(crate) kind: ParseErrorKind,

    /// The position in the source file this error originated from.
    pub(crate) sourcepos: SourcePosition,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.sourcepos.start.line, self.sourcepos.start.column, self.kind)
    }
}

impl ParseError {
    fn new<S: Into<SourcePosition>>(kind: ParseErrorKind, sourcepos: S) -> Self {
        Self { kind, sourcepos: sourcepos.into() }
    }
}

/// The kind of error.
#[derive(Debug)]
pub(crate) enum ParseErrorKind {
    /// We don't support parsing this element.
    UnsupportedElement(&'static str),

    /// We don't support parsing an element in a specific container.
    UnsupportedStructure { container: &'static str, element: &'static str },

    /// We don't support unfenced code blocks.
    UnfencedCodeBlock,

    /// A code block contains invalid attributes.
    InvalidCodeBlock(CodeBlockParseError),

    /// An internal parsing error.
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
            Self::InvalidCodeBlock(error) => write!(f, "invalid code block: {error}"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
        }
    }
}

impl ParseErrorKind {
    fn with_sourcepos<S: Into<SourcePosition>>(self, sourcepos: S) -> ParseError {
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
            NodeValue::MultilineBlockQuote(_) => "multiline block quote",
            NodeValue::Math(_) => "math",
            NodeValue::Escaped => "escaped",
            NodeValue::WikiLink(_) => "wiki link",
            NodeValue::Underline => "underline",
            NodeValue::SpoileredText => "spoilered text",
            NodeValue::EscapedTag(_) => "escaped tag",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::markdown::elements::SnippetLanguage;
    use rstest::rstest;
    use std::path::Path;

    fn try_parse(input: &str) -> Result<Vec<MarkdownElement>, ParseError> {
        let arena = Arena::new();
        MarkdownParser::new(&arena).parse(input, &mut Resources::default())
    }

    fn parse_single(input: &str) -> MarkdownElement {
        let elements = try_parse(input).expect("failed to parse");
        assert_eq!(elements.len(), 1, "more than one element: {elements:?}");
        elements.into_iter().next().unwrap()
    }

    fn parse_all(input: &str) -> Vec<MarkdownElement> {
        try_parse(input).expect("parsing failed")
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
            Text::from("some "),
            Text::new("bold text", TextStyle::default().bold()),
            Text::from(", "),
            Text::new("italics", TextStyle::default().italics()),
            Text::from(", "),
            Text::new("italics", TextStyle::default().italics()),
            Text::from(", "),
            Text::new("nested ", TextStyle::default().bold()),
            Text::new("italics", TextStyle::default().italics().bold()),
            Text::from(", "),
            Text::new("strikethrough", TextStyle::default().strikethrough()),
        ];

        let expected_elements = &[ParagraphElement::Text(TextBlock(expected_chunks))];
        assert_eq!(elements, expected_elements);
    }

    #[test]
    fn link() {
        let parsed = parse_single("my [website](https://example.com)");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks = vec![Text::from("my "), Text::new("https://example.com", TextStyle::default().link())];

        let expected_elements = &[ParagraphElement::Text(TextBlock(expected_chunks))];
        assert_eq!(elements, expected_elements);
    }

    #[test]
    fn image() {
        let parsed = parse_single("![](potato.png)");
        let MarkdownElement::Image { path, .. } = parsed else { panic!("not an image: {parsed:?}") };
        assert_eq!(path, Path::new("potato.png"));
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
        let expected_chunks = [Text::from("Title")];
        assert_eq!(text.0, expected_chunks);
    }

    #[test]
    fn heading() {
        let parsed = parse_single("# Title **with bold**");
        let MarkdownElement::Heading { text, level } = parsed else { panic!("not a heading: {parsed:?}") };
        let expected_chunks = vec![Text::from("Title "), Text::new("with bold", TextStyle::default().bold())];

        assert_eq!(level, 1);
        assert_eq!(text.0, expected_chunks);
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

        let expected_chunks = &[Text::from("some text"), Text::from(" "), Text::from("with line breaks")];
        let ParagraphElement::Text(text) = &elements[0] else { panic!("non-text in paragraph") };
        assert_eq!(text.0, expected_chunks);
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
        let MarkdownElement::Snippet(code) = parsed else { panic!("not a code block: {parsed:?}") };
        assert_eq!(code.language, SnippetLanguage::Rust);
        assert_eq!(code.contents, "let q = 42;\n");
        assert!(!code.attributes.execute);
    }

    #[test]
    fn executable_code_block() {
        let parsed = parse_single(
            r"
```bash +exec
echo hi mom
````
",
        );
        let MarkdownElement::Snippet(code) = parsed else { panic!("not a code block: {parsed:?}") };
        assert_eq!(code.language, SnippetLanguage::Bash);
        assert!(code.attributes.execute);
    }

    #[test]
    fn inline_code() {
        let parsed = parse_single("some `inline code`");
        let MarkdownElement::Paragraph(elements) = parsed else { panic!("not a paragraph: {parsed:?}") };
        let expected_chunks = &[Text::from("some "), Text::new("inline code", TextStyle::default().code())];
        assert_eq!(elements.len(), 1);

        let ParagraphElement::Text(text) = &elements[0] else { panic!("non-text in paragraph") };
        assert_eq!(text.0, expected_chunks);
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
        let MarkdownElement::Comment { comment, .. } = parsed else { panic!("not a comment: {parsed:?}") };
        assert_eq!(comment, " foo ");
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
            r#"
> bar!@#$%^&*()[]'"{}-=`~,.<>/?
> foo
> 
> * a
> * b
"#,
        );
        let MarkdownElement::BlockQuote(lines) = parsed else { panic!("not a block quote: {parsed:?}") };
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "bar!@#$%^&*()[]'\"{}-=`~,.<>/?");
        assert_eq!(lines[1], "foo");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "* a");
        assert_eq!(lines[4], "* b");
    }

    #[test]
    fn multiline_block_quote() {
        let parsed = parse_single(
            r"
>>>
bar
foo

* a
* b
>>>",
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

    #[test]
    fn error_lines_offset_by_front_matter() {
        let input = r"---
hi
mom
---

* ![](potato.png)
";
        let arena = Arena::new();
        let result = MarkdownParser::new(&arena).parse(input, &mut Resources::default());
        let Err(e) = result else {
            panic!("parsing didn't fail");
        };
        assert_eq!(e.sourcepos.start.line, 5);
        assert_eq!(e.sourcepos.start.column, 3);
    }

    #[test]
    fn comment_lines_offset_by_front_matter() {
        let parsed = parse_all(
            r"---
hi
mom
---

<!-- hello -->
",
        );
        let MarkdownElement::Comment { source_position, .. } = &parsed[1] else { panic!("not a comment") };
        assert_eq!(source_position.start.line, 5);
        assert_eq!(source_position.start.column, 1);
    }

    #[rstest]
    #[case::lf("\n")]
    #[case::crlf("\r\n")]
    fn front_matter_newlines(#[case] nl: &str) {
        let input = format!("---{nl}hi{nl}mom{nl}---{nl}");
        let parsed = parse_single(&input);
        let MarkdownElement::FrontMatter(contents) = &parsed else { panic!("not a front matter") };

        let expected = format!("hi{nl}mom{nl}");
        assert_eq!(contents, &expected);
    }
}
