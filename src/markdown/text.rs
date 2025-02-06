use super::{
    elements::{Line, Text},
    text_style::TextStyle,
};
use std::mem;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A weighted line of text.
///
/// The weight of a character is its given by its width in unicode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WeightedLine {
    text: Vec<WeightedText>,
    width: usize,
    height: u8,
}

impl WeightedLine {
    /// Split this line into chunks of at most `max_length` width.
    pub(crate) fn split(&self, max_length: usize) -> SplitTextIter {
        SplitTextIter::new(&self.text, max_length)
    }

    /// The total width of this line.
    pub(crate) fn width(&self) -> usize {
        self.width
    }

    /// The height of this line.
    pub(crate) fn height(&self) -> u8 {
        self.height
    }

    /// Get an iterator to the underlying text chunks.
    #[cfg(test)]
    pub(crate) fn iter_texts(&self) -> impl Iterator<Item = &WeightedText> {
        self.text.iter()
    }
}

impl From<Line> for WeightedLine {
    fn from(block: Line) -> Self {
        block.0.into()
    }
}

impl From<Vec<Text>> for WeightedLine {
    fn from(mut texts: Vec<Text>) -> Self {
        let mut output = Vec::new();
        let mut index = 0;
        let mut width = 0;
        let mut height = 1;
        // Compact chunks so any consecutive chunk with the same style is merged into the same block.
        while index < texts.len() {
            let mut target = mem::replace(&mut texts[index], Text::from(""));
            let mut current = index + 1;
            while current < texts.len() && texts[current].style == target.style {
                let current_content = mem::take(&mut texts[current].content);
                target.content.push_str(&current_content);
                current += 1;
            }
            let size = target.style.size.max(1);
            width += target.content.width() * size as usize;
            output.push(target.into());
            index = current;
            height = height.max(size);
        }
        Self { text: output, width, height }
    }
}

impl From<String> for WeightedLine {
    fn from(text: String) -> Self {
        let width = text.width();
        let text = vec![WeightedText::from(text)];
        Self { text, width, height: 1 }
    }
}

impl From<&str> for WeightedLine {
    fn from(text: &str) -> Self {
        Self::from(text.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CharAccumulator {
    width: usize,
    bytes: usize,
}

/// A piece of weighted text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WeightedText {
    text: Text,
    accumulators: Vec<CharAccumulator>,
}

impl WeightedText {
    fn to_ref(&self) -> WeightedTextRef {
        WeightedTextRef { text: &self.text.content, accumulators: &self.accumulators, style: self.text.style }
    }

    pub(crate) fn width(&self) -> usize {
        self.to_ref().width()
    }

    pub(crate) fn text(&self) -> &Text {
        &self.text
    }
}

impl<S: Into<String>> From<S> for WeightedText {
    fn from(text: S) -> Self {
        Self::from(Text::from(text.into()))
    }
}

impl From<Text> for WeightedText {
    fn from(text: Text) -> Self {
        let mut accumulators = Vec::new();
        let mut width = 0;
        let mut bytes = 0;
        for c in text.content.chars() {
            accumulators.push(CharAccumulator { width, bytes });
            width += c.width().unwrap_or(0);
            bytes += c.len_utf8();
        }
        accumulators.push(CharAccumulator { width, bytes });
        Self { text, accumulators }
    }
}

/// An iterator over the chunks in a [WeightedLine].
pub(crate) struct SplitTextIter<'a> {
    texts: &'a [WeightedText],
    max_length: usize,
    current: Option<WeightedTextRef<'a>>,
}

impl<'a> SplitTextIter<'a> {
    fn new(texts: &'a [WeightedText], max_length: usize) -> Self {
        Self { texts, max_length, current: texts.first().map(WeightedText::to_ref) }
    }
}

impl<'a> Iterator for SplitTextIter<'a> {
    type Item = Vec<WeightedTextRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.as_ref()?;

        let mut elements = Vec::new();
        let mut remaining = self.max_length as i64;
        while let Some(current) = self.current.take() {
            let (head, rest) = current.word_split_at_length(remaining as usize);
            // Prevent splitting a word partially. We do allow this on the first chunk as otherwise
            // a word longer than `max_length` would never be split.
            if !rest.text.is_empty() && !rest.text.starts_with(' ') && !elements.is_empty() {
                self.current = Some(current);
                break;
            }
            let head_width = head.width();
            remaining -= head_width as i64;
            elements.push(head);

            // The moment we hit a chunk we couldn't fully split, we're done.
            if !rest.text.is_empty() {
                self.current = Some(rest.trim_start());
                break;
            }

            // Consume the first one and point to the next one, if any.
            self.texts = &self.texts[1..];
            self.current = self.texts.first().map(WeightedText::to_ref);
        }
        Some(elements)
    }
}

/// A reference of a piece of a [WeightedText].
#[derive(Clone, Debug)]
pub(crate) struct WeightedTextRef<'a> {
    text: &'a str,
    accumulators: &'a [CharAccumulator],
    style: TextStyle,
}

impl<'a> WeightedTextRef<'a> {
    /// Decompose this into its parts.
    pub(crate) fn into_parts(self) -> (&'a str, TextStyle) {
        (self.text, self.style)
    }

    // Attempts to split this at a word boundary.
    //
    // This will try to consume as many words as possible up to the given maximum length, and
    // return the text before and after that split point.
    fn word_split_at_length(&self, max_length: usize) -> (Self, Self) {
        if self.width() <= max_length {
            return (self.make_ref(0, self.text.len()), self.make_ref(0, 0));
        }

        let target_chunk = self.substr(max_length + 1);
        let output_chunk = match target_chunk.rsplit_once(' ') {
            Some((before, _)) => before,
            None => self.substr(max_length),
        };
        (self.make_ref(0, output_chunk.len()), self.make_ref(output_chunk.len(), self.text.len()))
    }

    fn substr(&self, max_length: usize) -> &'a str {
        let last_index = self.bytes_until(max_length);
        &self.text[0..last_index]
    }

    fn make_ref(&self, from: usize, to: usize) -> Self {
        let text = &self.text[from..to];
        let leading_char_count = self.text[0..from].chars().count();
        let output_char_count = text.chars().count();
        let character_lengths = &self.accumulators[leading_char_count..leading_char_count + output_char_count + 1];
        WeightedTextRef { text, accumulators: character_lengths, style: self.style }
    }

    fn trim_start(self) -> Self {
        let text = self.text.trim_start();
        let trimmed = self.text.chars().count() - text.chars().count();
        let accumulators = &self.accumulators[trimmed..];
        Self { text, accumulators, style: self.style }
    }

    pub(crate) fn width(&self) -> usize {
        let last_width = self.accumulators.last().map(|a| a.width).unwrap_or(0);
        let first_width = self.accumulators.first().map(|a| a.width).unwrap_or(0);
        last_width - first_width
    }

    fn bytes_until(&self, index: usize) -> usize {
        let last_bytes =
            self.accumulators.get(index).or_else(|| self.accumulators.last()).map(|a| a.bytes).unwrap_or(0);
        let first_bytes = self.accumulators.first().map(|a| a.bytes).unwrap_or(0);
        last_bytes - first_bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    fn join_lines<'a>(lines: impl Iterator<Item = Vec<WeightedTextRef<'a>>>) -> Vec<String> {
        lines.map(|l| l.iter().map(|weighted| weighted.text).collect::<Vec<_>>().join(" ")).collect()
    }

    #[test]
    fn text_creation() {
        let text = WeightedText::from("hello world");

        let text_ref = text.to_ref();
        assert_eq!(text_ref.width(), 11);
    }

    #[test]
    fn text_creation_utf8() {
        let text = WeightedText::from("█████");

        let text_ref = text.to_ref();
        assert_eq!(text_ref.width(), 5);
        assert_eq!(text_ref.bytes_until(0), 0);
        assert_eq!(text_ref.bytes_until(1), 3);
        assert_eq!(text_ref.bytes_until(2), 6);
        assert_eq!(text_ref.bytes_until(3), 9);
        assert_eq!(text_ref.bytes_until(4), 12);

        let text_ref = text_ref.make_ref(3, 12);
        assert_eq!(text_ref.width(), 3);
        assert_eq!(text_ref.bytes_until(0), 0);
        assert_eq!(text_ref.bytes_until(1), 3);
        assert_eq!(text_ref.bytes_until(2), 6);

        let text_ref = text_ref.make_ref(0, 9);
        assert_eq!(text_ref.width(), 3);
        assert_eq!(text_ref.bytes_until(0), 0);
        assert_eq!(text_ref.bytes_until(1), 3);
        assert_eq!(text_ref.bytes_until(2), 6);
    }

    #[test]
    fn minimal_split() {
        let text = WeightedText::from("█████");
        let text_ref = text.to_ref();
        let (head, rest) = text_ref.word_split_at_length(1);
        assert_eq!(head.width(), 1);
        assert_eq!(rest.width(), 4);
    }

    #[test]
    fn no_spaces_split() {
        let text = WeightedText::from("█████");
        let text_ref = text.to_ref();
        let (head, rest) = text_ref.word_split_at_length(2);
        assert_eq!(head.width(), 2);
        assert_eq!(rest.width(), 3);
    }

    #[test]
    fn make_ref() {
        let text = WeightedText::from("hello world");
        let text_ref = text.to_ref();
        let head = text_ref.make_ref(0, 1);
        assert_eq!(head.text, "h");
        assert_eq!(head.width(), 1);

        let rest = text_ref.make_ref(1, 11);
        assert_eq!(rest.text, "ello world");
        assert_eq!(rest.width(), 10);
    }

    #[test]
    fn word_split() {
        let text = WeightedText::from("short string");
        let (head, rest) = text.to_ref().word_split_at_length(7);
        assert_eq!(head.text, "short");
        assert_eq!(rest.text, " string");
    }

    #[test]
    fn split_at_full_length() {
        let text = WeightedLine::from("hello world");
        let lines = join_lines(text.split(11));
        let expected = vec!["hello world"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn no_split_necessary() {
        let text =
            WeightedLine { text: vec![WeightedText::from("short"), WeightedText::from("text")], width: 0, height: 1 };
        let lines = join_lines(text.split(50));
        let expected = vec!["short text"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn split_lines_single() {
        let text = WeightedLine { text: vec![WeightedText::from("this is a slightly long line")], width: 0, height: 1 };
        let lines = join_lines(text.split(6));
        let expected = vec!["this", "is a", "slight", "ly", "long", "line"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn split_lines_multi() {
        let text = WeightedLine {
            text: vec![
                WeightedText::from("this is a slightly long line"),
                WeightedText::from("another chunk"),
                WeightedText::from("yet some other piece"),
            ],
            width: 0,
            height: 1,
        };
        let lines = join_lines(text.split(10));
        let expected = vec!["this is a", "slightly", "long line", "another", "chunk yet", "some other", "piece"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn long_splits() {
        let text = WeightedLine {
            text: vec![
                WeightedText::from("this is a slightly long line"),
                WeightedText::from("another chunk"),
                WeightedText::from("yet some other piece"),
            ],
            width: 0,
            height: 1,
        };
        let lines = join_lines(text.split(50));
        let expected = vec!["this is a slightly long line another chunk yet some", "other piece"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn prefixed_by_whitespace() {
        let text = WeightedLine::from("   * bullet");
        let lines = join_lines(text.split(50));
        let expected = vec!["   * bullet"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn utf8_character() {
        let text = WeightedLine::from("• A");
        let lines = join_lines(text.split(50));
        let expected = vec!["• A"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn many_utf8_characters() {
        let content = "█████ ██";
        let text = WeightedLine::from(content);
        let lines = join_lines(text.split(3));
        let expected = vec!["███", "██", "██"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn no_whitespaces_ascii() {
        let content = "X".repeat(10);
        let text = WeightedLine::from(content);
        let lines = join_lines(text.split(3));
        let expected = vec!["XXX", "XXX", "XXX", "X"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn no_whitespaces_utf8() {
        let content = "─".repeat(10);
        let text = WeightedLine::from(content);
        let lines = join_lines(text.split(3));
        let expected = vec!["───", "───", "───", "─"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn wide_characters() {
        let content = "Ｈｅｌｌｏ ｗｏｒｌｄ";
        let text = WeightedLine::from(content);
        let lines = join_lines(text.split(10));
        // Each word is 10 characters long
        let expected = vec!["Ｈｅｌｌｏ", "ｗｏｒｌｄ"];
        assert_eq!(lines, expected);
    }

    #[rstest]
    #[case::single(&["hello".into()], 1)]
    #[case::two(&["hello".into(), " world".into()], 1)]
    #[case::three(&["hello".into(), " ".into(), "world".into()], 1)]
    #[case::split(&["hello".into(), Text::new(" ", TextStyle::default().bold()), "world".into()], 3)]
    #[case::split_merged(&["hello".into(), Text::new(" ", TextStyle::default().bold()), Text::new("w", TextStyle::default().bold()), "orld".into()], 3)]
    fn compaction(#[case] texts: &[Text], #[case] expected: usize) {
        let block = WeightedLine::from(texts.to_vec());
        assert_eq!(block.text.len(), expected);
    }
}
