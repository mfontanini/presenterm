use tl::Node;

pub(crate) struct HtmlBlockParser;

impl HtmlBlockParser {
    pub(crate) fn parse(&self, input: &str) -> Result<Vec<HtmlBlock>, ParseHtmlBlockError> {
        let dom = tl::parse(input, Default::default())?;
        let parser = dom.parser();
        let children = dom.children();
        let mut output = Vec::new();
        for child in children {
            let child = child.get(parser).expect("faild to get");
            match child {
                Node::Tag(tag) => match tag.name().as_bytes() {
                    b"speaker-note" => {
                        let contents = child.inner_text(parser);
                        let text = if let Some(text) = contents.strip_prefix('\n') { text } else { &contents };
                        let lines = text.lines().map(|l| l.to_string()).collect();
                        output.push(HtmlBlock::SpeakerNotes { lines });
                    }
                    name => return Err(ParseHtmlBlockError::UnsupportedTag(String::from_utf8_lossy(name).to_string())),
                },
                Node::Comment(bytes) => {
                    let start_tag = "<!--";
                    let end_tag = "-->";
                    let text = bytes.as_utf8_str();
                    let text = &text[start_tag.len()..];
                    let text = &text[0..text.len() - end_tag.len()];
                    output.push(HtmlBlock::Comment(text.to_string()));
                }
                Node::Raw(_) => (),
            };
        }
        Ok(output)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum HtmlBlock {
    SpeakerNotes { lines: Vec<String> },
    Comment(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseHtmlBlockError {
    #[error("parsing html failed: {0}")]
    ParsingHtml(#[from] tl::ParseError),

    #[error("unsupported HTML tag: {0}")]
    UnsupportedTag(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_note() {
        let input = r"
<speaker-note>
hello

this is
text
</speaker-note>
";
        let mut blocks = HtmlBlockParser.parse(&input).expect("parse failed");
        assert_eq!(blocks.len(), 1);

        let HtmlBlock::SpeakerNotes { lines } = blocks.pop().unwrap() else {
            panic!("not a speaker note!");
        };
        let expected_lines = &["hello", "", "this is", "text"];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn comment() {
        let input = r"
<!-- this is a comment -->
";
        let mut blocks = HtmlBlockParser.parse(&input).expect("parse failed");
        assert_eq!(blocks.len(), 1);

        let HtmlBlock::Comment(comment) = blocks.pop().unwrap() else {
            panic!("not a comment!");
        };
        assert_eq!(comment, " this is a comment ");
    }

    #[test]
    fn multiple_blocks() {
        let input = r"
<speaker-note>hello</speaker-note>
<!-- note -->
";
        let blocks = HtmlBlockParser.parse(&input).expect("parse failed");
        assert_eq!(blocks.len(), 2);
    }
}
