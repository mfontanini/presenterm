use super::elements::StyledText;
use crate::style::TextStyle;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Debug, Default)]
pub struct WeightedLine(Vec<WeightedText>);

impl WeightedLine {
    pub fn split(&self, max_length: usize) -> SplitTextIter {
        SplitTextIter::new(&self.0, max_length)
    }

    pub fn width(&self) -> usize {
        self.0.iter().map(|text| text.width()).sum()
    }
}

impl From<Vec<WeightedText>> for WeightedLine {
    fn from(texts: Vec<WeightedText>) -> Self {
        Self(texts)
    }
}

#[derive(Clone, Debug)]
struct CharAccumulator {
    width: usize,
    bytes: usize,
}

#[derive(Clone, Debug)]
pub struct WeightedText {
    text: StyledText,
    accumulators: Vec<CharAccumulator>,
}

impl WeightedText {
    fn to_ref(&self) -> WeightedTextRef {
        WeightedTextRef { text: &self.text.text, accumulators: &self.accumulators, style: self.text.style.clone() }
    }

    fn width(&self) -> usize {
        self.accumulators.last().map(|a| a.width).unwrap_or(0)
    }
}

impl From<StyledText> for WeightedText {
    fn from(text: StyledText) -> Self {
        let mut accumulators = Vec::new();
        let mut width = 0;
        let mut bytes = 0;
        for c in text.text.chars() {
            width += c.width().unwrap_or(0);
            bytes += c.len_utf8();
            accumulators.push(CharAccumulator { width, bytes });
        }
        Self { text, accumulators }
    }
}

pub struct SplitTextIter<'a> {
    texts: &'a [WeightedText],
    max_length: usize,
    current: Option<WeightedTextRef<'a>>,
}

impl<'a> SplitTextIter<'a> {
    fn new(texts: &'a [WeightedText], max_length: usize) -> Self {
        Self { texts, max_length, current: texts.get(0).map(WeightedText::to_ref) }
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
            self.current = self.texts.get(0).map(WeightedText::to_ref);
        }
        Some(elements)
    }
}

#[derive(Clone, Debug)]
pub struct WeightedTextRef<'a> {
    text: &'a str,
    accumulators: &'a [CharAccumulator],
    style: TextStyle,
}

impl<'a> WeightedTextRef<'a> {
    pub fn into_parts(self) -> (&'a str, TextStyle) {
        (self.text, self.style)
    }

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
        let max_length = self.bytes_until(max_length);
        &self.text[0..max_length]
    }

    fn make_ref(&self, from: usize, to: usize) -> Self {
        let text = &self.text[from..to];
        let from_char_count = self.text[0..from].chars().count();
        let to_char_count = self.text[from..to].chars().count();
        let character_lengths = &self.accumulators[from_char_count..from_char_count + to_char_count];
        WeightedTextRef { text, accumulators: character_lengths, style: self.style.clone() }
    }

    fn trim_start(self) -> Self {
        let text = self.text.trim_start();
        let trimmed = self.text.chars().count() - text.chars().count();
        let accumulators = &self.accumulators[trimmed..];
        Self { text, accumulators, style: self.style }
    }

    fn width(&self) -> usize {
        let last_width = self.accumulators.last().map(|a| a.width).unwrap_or(0);
        let first_width = self.accumulators.get(0).map(|a| a.width).unwrap_or(0);
        last_width - first_width + 1
    }

    fn bytes_until(&self, index: usize) -> usize {
        let last_bytes =
            self.accumulators.get(index).or_else(|| self.accumulators.last()).map(|a| a.bytes).unwrap_or(0);
        let first_bytes = self.accumulators.get(0).map(|a| a.bytes).unwrap_or(0);
        last_bytes - first_bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn join_lines<'a>(lines: impl Iterator<Item = Vec<WeightedTextRef<'a>>>) -> Vec<String> {
        lines.map(|l| l.iter().map(|weighted| weighted.text).collect::<Vec<_>>().join(" ")).collect()
    }

    #[test]
    fn word_split() {
        let text = WeightedText::from(StyledText::plain("short string"));
        let (head, rest) = text.to_ref().word_split_at_length(7);
        assert_eq!(head.text, "short");
        assert_eq!(rest.text, " string");
        assert_eq!(head.accumulators.len(), 5);
        assert_eq!(rest.accumulators.len(), 7);
    }

    #[test]
    fn no_split_necessary() {
        let text = WeightedLine(vec![
            WeightedText::from(StyledText::plain("short")),
            WeightedText::from(StyledText::plain("text")),
        ]);
        let lines = join_lines(text.split(50));
        let expected = vec!["short text"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn split_lines_single() {
        let text = WeightedLine(vec![WeightedText::from(StyledText::plain("this is a slightly long line"))]);
        let lines = join_lines(text.split(6));
        let expected = vec!["this", "is a", "slight", "ly", "long", "line"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn split_lines_multi() {
        let text = WeightedLine(vec![
            WeightedText::from(StyledText::plain("this is a slightly long line")),
            WeightedText::from(StyledText::plain("another chunk")),
            WeightedText::from(StyledText::plain("yet some other piece")),
        ]);
        let lines = join_lines(text.split(10));
        let expected = vec!["this is a", "slightly", "long line", "another", "chunk yet", "some other", "piece"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn long_splits() {
        let text = WeightedLine(vec![
            WeightedText::from(StyledText::plain("this is a slightly long line")),
            WeightedText::from(StyledText::plain("another chunk")),
            WeightedText::from(StyledText::plain("yet some other piece")),
        ]);
        let lines = join_lines(text.split(50));
        let expected = vec!["this is a slightly long line another chunk yet some", "other piece"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn prefixed_by_whitespace() {
        let text = WeightedLine(vec![WeightedText::from(StyledText::plain("   * bullet"))]);
        let lines = join_lines(text.split(50));
        let expected = vec!["   * bullet"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn utf8_character() {
        let text = WeightedLine(vec![WeightedText::from(StyledText::plain("• A"))]);
        let lines = join_lines(text.split(50));
        let expected = vec!["• A"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn only_utf8_characters() {
        let content = "─".repeat(10);
        let text = WeightedLine(vec![WeightedText::from(StyledText::plain(content))]);
        let lines = join_lines(text.split(3));
        let expected = vec!["───", "───", "───", "─"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn wide_characters() {
        let content = "Ｈｅｌｌｏ ｗｏｒｌｄ";
        let text = WeightedLine(vec![WeightedText::from(StyledText::plain(content))]);
        let lines = join_lines(text.split(10));
        // Each word is 10 characters long
        let expected = vec!["Ｈｅｌｌｏ", "ｗｏｒｌｄ"];
        assert_eq!(lines, expected);
    }
}
