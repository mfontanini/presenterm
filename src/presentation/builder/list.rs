use crate::{
    markdown::{
        elements::{ListItem, ListItemType, Text},
        text_style::TextStyle,
    },
    presentation::builder::{BuildResult, LastElement, PresentationBuilder},
    render::operation::{BlockLine, RenderOperation},
};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_list(&mut self, list: Vec<ListItem>) -> BuildResult {
        let last_chunk_operation = self.slide_chunks.last().and_then(|chunk| chunk.iter_operations().last());
        // If the last chunk ended in a list, pop the newline so we get them all next to each
        // other.
        if matches!(last_chunk_operation, Some(RenderOperation::RenderLineBreak))
            && self.slide_state.last_chunk_ended_in_list
            && self.chunk_operations.is_empty()
        {
            self.slide_chunks.last_mut().unwrap().pop_last();
        }
        // If this chunk just starts (because there was a pause), pick up from the last index.
        let start_index = match self.slide_state.last_element {
            LastElement::List { last_index } if self.chunk_operations.is_empty() => last_index + 1,
            _ => 0,
        };

        let block_length =
            list.iter().map(|l| self.list_item_prefix(l).width() + l.contents.width()).max().unwrap_or_default() as u16;
        let block_length = block_length * self.slide_font_size() as u16;
        let incremental_lists = self.slide_state.incremental_lists.unwrap_or(self.options.incremental_lists);
        let iter = ListIterator::new(list, start_index);
        if incremental_lists && self.options.pause_before_incremental_lists {
            self.push_pause();
        }
        for (index, item) in iter.enumerate() {
            if index > 0 && incremental_lists {
                self.push_pause();
            }
            self.push_list_item(item.index, item.item, block_length)?;
        }
        if incremental_lists && self.options.pause_after_incremental_lists {
            self.push_pause();
        }
        Ok(())
    }

    fn push_list_item(&mut self, index: usize, item: ListItem, block_length: u16) -> BuildResult {
        let prefix = self.list_item_prefix(&item);
        let mut text = item.contents.resolve(&self.theme.palette)?;
        let font_size = self.slide_font_size();
        for piece in &mut text.0 {
            self.apply_theme_text_style(piece);
            piece.style = piece.style.size(font_size);
        }
        let alignment = self.slide_state.alignment.unwrap_or_default();
        self.chunk_operations.push(RenderOperation::RenderBlockLine(BlockLine {
            prefix: prefix.into(),
            right_padding_length: 0,
            repeat_prefix_on_wrap: false,
            text: text.into(),
            block_length,
            alignment,
            block_color: None,
        }));
        let newlines = self.slide_state.list_item_newlines.unwrap_or(self.options.list_item_newlines);
        self.push_line_breaks(newlines as usize);
        if item.depth == 0 {
            self.slide_state.last_element = LastElement::List { last_index: index };
        }
        Ok(())
    }

    fn list_item_prefix(&self, item: &ListItem) -> Text {
        let font_size = self.slide_font_size();
        let spaces_per_indent = match item.depth {
            0 => 3_u8.div_ceil(font_size),
            _ => {
                if font_size == 1 {
                    3
                } else {
                    2
                }
            }
        };
        let padding_length = (item.depth as usize + 1) * spaces_per_indent as usize;
        let mut prefix: String = " ".repeat(padding_length);
        match item.item_type {
            ListItemType::Unordered => {
                let delimiter = match item.depth {
                    0 => '•',
                    1 => '◦',
                    _ => '▪',
                };
                prefix.push(delimiter);
                prefix.push_str("  ");
            }
            ListItemType::OrderedParens(value) => {
                prefix.push_str(&value.to_string());
                prefix.push_str(") ");
            }
            ListItemType::OrderedPeriod(value) => {
                prefix.push_str(&value.to_string());
                prefix.push_str(". ");
            }
        };
        Text::new(prefix, TextStyle::default().size(font_size))
    }
}

struct ListIterator<I> {
    remaining: I,
    next_index: usize,
    current_depth: u8,
    saved_indexes: Vec<usize>,
}

impl<I> ListIterator<I> {
    fn new<T>(remaining: T, next_index: usize) -> Self
    where
        I: Iterator<Item = ListItem>,
        T: IntoIterator<IntoIter = I, Item = ListItem>,
    {
        Self { remaining: remaining.into_iter(), next_index, current_depth: 0, saved_indexes: Vec::new() }
    }
}

impl<I> Iterator for ListIterator<I>
where
    I: Iterator<Item = ListItem>,
{
    type Item = IndexedListItem;

    fn next(&mut self) -> Option<Self::Item> {
        let head = self.remaining.next()?;
        if head.depth != self.current_depth {
            if head.depth > self.current_depth {
                // If we're going deeper, save the next index so we can continue later on and start
                // from 0.
                self.saved_indexes.push(self.next_index);
                self.next_index = 0;
            } else {
                // if we're getting out, recover the index we had previously saved.
                for _ in head.depth..self.current_depth {
                    self.next_index = self.saved_indexes.pop().unwrap_or(0);
                }
            }
            self.current_depth = head.depth;
        }
        let index = self.next_index;
        self.next_index += 1;
        Some(IndexedListItem { index, item: head })
    }
}

#[derive(Debug)]
struct IndexedListItem {
    index: usize,
    item: ListItem,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::builder::{PresentationBuilderOptions, utils::Test};
    use rstest::rstest;
    use std::iter;

    #[test]
    fn iterate_list() {
        let iter = ListIterator::new(
            vec![
                ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "1".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "00".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "01".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 1, contents: "02".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 2, contents: "001".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "2".into(), item_type: ListItemType::Unordered },
            ],
            0,
        );
        let expected_indexes = [0, 1, 0, 1, 2, 0, 2];
        let indexes: Vec<_> = iter.map(|item| item.index).collect();
        assert_eq!(indexes, expected_indexes);
    }

    #[test]
    fn iterate_list_starting_from_other() {
        let list = ListIterator::new(
            vec![
                ListItem { depth: 0, contents: "0".into(), item_type: ListItemType::Unordered },
                ListItem { depth: 0, contents: "1".into(), item_type: ListItemType::Unordered },
            ],
            3,
        );
        let expected_indexes = [3, 4];
        let indexes: Vec<_> = list.into_iter().map(|item| item.index).collect();
        assert_eq!(indexes, expected_indexes);
    }

    #[test]
    fn unordered() {
        let input = "
* A
    * AA
        * AAA
    * AB
* B
    * BA 
";
        let lines = Test::new(input).render().rows(7).columns(16).into_lines();
        let expected = &[
            "                ",
            "   •  A         ",
            "      ◦  AA     ",
            "         ▪  AAA ",
            "      ◦  AB     ",
            "   •  B         ",
            "      ◦  BA     ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn unordered_paused() {
        let input = "
* A
<!-- pause -->
* B
<!-- pause -->
* C
";
        let lines = Test::new(input).render().rows(4).columns(8).into_lines();
        let expected = &["        ", "   •  A ", "   •  B ", "   •  C "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn ordered_period() {
        let input = "
1. A
    1. AA
        1. AAA
    2. AB
2. B
    1. BA 
";
        let lines = Test::new(input).render().rows(7).columns(16).into_lines();
        let expected = &[
            "                ",
            "   1. A         ",
            "      1. AA     ",
            "         1. AAA ",
            "      2. AB     ",
            "   2. B         ",
            "      1. BA     ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn ordered_parens() {
        let input = "
1) A
    1) AA
2) B
";
        let lines = Test::new(input).render().rows(4).columns(12).into_lines();
        let expected = &["            ", "   1) A     ", "      1) AA ", "   2) B     "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn ordered_paused() {
        let input = "
1. A
<!-- pause -->
2. B
<!-- pause -->
3. C
";
        let lines = Test::new(input).render().rows(4).columns(8).into_lines();
        let expected = &["        ", "   1. A ", "   2. B ", "   3. C "];
        assert_eq!(lines, expected);
    }

    #[rstest]
    #[case::zero(0)]
    #[case::one(1)]
    #[case::two(2)]
    fn visible_pauses(#[case] advances: usize) {
        let input = "
* A
<!-- pause -->
* B
<!-- pause -->
* C
";
        let lines = Test::new(input).render().rows(4).columns(8).advances(advances).into_lines();
        let mut expected = vec!["        ", "   •  A "];
        if advances >= 1 {
            expected.push("   •  B ");
        }
        if advances >= 2 {
            expected.push("   •  C ");
        }
        expected.extend(iter::repeat_n("        ", 4 - expected.len()));
        assert_eq!(lines, expected);
    }

    #[rstest]
    #[case::first_no_before_no_after(true, true, 0, 0)]
    #[case::first_no_before(false, true, 0, 1)]
    #[case::second_no_before_no_after(true, true, 1, 1)]
    #[case::second_no_before(false, true, 1, 2)]
    #[case::second(false, false, 2, 4)]
    #[case::third_no_before_no_after(true, true, 2, 2)]
    #[case::third_no_before(false, true, 3, 4)]
    #[case::third_no_after(true, false, 3, 4)]
    fn incremental_lists(
        #[case] pause_before: bool,
        #[case] pause_after: bool,
        #[case] advances: usize,
        #[case] visible: usize,
    ) {
        let input = "
<!-- incremental_lists: true -->
* A
* B
* C

hi
";
        let options = PresentationBuilderOptions {
            pause_before_incremental_lists: pause_before,
            pause_after_incremental_lists: pause_after,
            ..Default::default()
        };
        let lines = Test::new(input).options(options).render().rows(6).columns(8).advances(advances).into_lines();
        let mut expected = vec!["        "];
        if visible >= 1 {
            expected.push("   •  A ");
        }
        if visible >= 2 {
            expected.push("   •  B ");
        }
        if visible >= 3 {
            expected.push("   •  C ");
        }
        if visible >= 4 {
            expected.push("        ");
            expected.push("hi      ");
        }
        expected.extend(iter::repeat_n("        ", 6 - expected.len()));
        assert_eq!(lines, expected);
    }

    #[test]
    fn font_size() {
        let input = "
<!-- font_size: 2 -->
* A
* B
";
        let lines = Test::new(input).render().rows(4).columns(12).into_lines();
        let expected = &["            ", "    •     A ", "            ", "    •     B "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn newlines() {
        let input = "
<!-- list_item_newlines: 2 -->
* A
* B
";
        let lines = Test::new(input).render().rows(4).columns(8).into_lines();
        let expected = &["        ", "   •  A ", "        ", "   •  B "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn incremental_lists_end_of_slide() {
        let input = "
<!-- incremental_lists: true -->
* A
* B

<!-- end_slide -->

other
";
        // 3 moves forward should land in the second slide, not an extra pause at the end
        let lines = Test::new(input).render().rows(4).columns(8).advances(3).into_lines();
        let expected = &["        ", "other   ", "        ", "        "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn pause_after_list() {
        let input = "
1. A

<!-- pause -->

# hi

2. B
";
        let lines = Test::new(input).render().rows(4).columns(8).advances(0).into_lines();
        let expected = &["        ", "   1. A ", "        ", "        "];
        assert_eq!(lines, expected);
    }
}
