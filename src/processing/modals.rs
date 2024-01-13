use crate::{
    markdown::{
        elements::{StyledText, Text},
        text::{WeightedLine, WeightedText},
    },
    presentation::{AsRenderOperations, MarginProperties, PresentationState, RenderOperation},
    processing::padding::pad_right,
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::Margin,
};
use std::{fmt::Display, rc::Rc};

#[derive(Default)]
pub(crate) struct IndexBuilder {
    titles: Vec<Text>,
}

impl IndexBuilder {
    pub(crate) fn add_title(&mut self, title: Text) {
        self.titles.push(title);
    }

    pub(crate) fn build(self, colors: Colors, state: PresentationState) -> Vec<RenderOperation> {
        if self.titles.is_empty() {
            return Vec::new();
        }
        let heading = "Slides";
        let longest_line = self.titles.iter().map(Text::width).max().unwrap_or(0) as u16;
        let longest_line = longest_line.max(heading.len() as u16);
        // Ensure we have a minimum width so it doesn't look too narrow.
        let longest_line = longest_line.max(12);
        let numbers_length = self.titles.len().ilog10() as usize + 1;
        // The final text looks like "| <number>: <content> |"
        let content_width = longest_line + numbers_length as u16 + 6;
        let mut prefix = vec![RenderOperation::SetColors(colors)];

        prefix.extend(Self::make_horizontal_border(content_width, '┌', '┐'));
        prefix.extend([
            RenderOperation::RenderText {
                line: Self::build_line(heading, [], content_width),
                alignment: Default::default(),
            },
            RenderOperation::RenderLineBreak,
        ]);
        prefix.extend(Self::make_horizontal_border(content_width, '├', '┤'));
        let mut titles = Vec::new();
        for (index, title) in self.titles.into_iter().enumerate() {
            let index = pad_right(index + 1, numbers_length);
            titles.push(Self::build_line(format!("{index}:"), title.chunks, content_width));
        }
        let suffix = Self::make_horizontal_border(content_width, '└', '┘').into_iter().collect();
        let drawer = IndexDrawer { prefix, titles, suffix, state, content_width };
        vec![RenderOperation::RenderDynamic(Rc::new(drawer))]
    }

    fn build_line<S, C>(prefix: S, text_chunks: C, content_width: u16) -> WeightedLine
    where
        S: Display,
        C: IntoIterator<Item = StyledText>,
    {
        let mut chunks = vec![WeightedText::from(format!("│ {prefix} "))];
        chunks.extend(text_chunks.into_iter().map(WeightedText::from));
        let missing = content_width as usize - 1 - chunks.iter().map(|c| c.width()).sum::<usize>();
        chunks.extend([WeightedText::from(" ".repeat(missing)), WeightedText::from("│")]);

        WeightedLine::from(chunks)
    }

    fn make_horizontal_border(content_length: u16, opening: char, closing: char) -> [RenderOperation; 2] {
        let mut line = String::from(opening);
        line.push_str(&"─".repeat((content_length - 2) as usize));
        line.push(closing);
        let horizontal_border = WeightedLine::from(vec![WeightedText::from(line)]);
        [
            RenderOperation::RenderText { line: horizontal_border.clone(), alignment: Default::default() },
            RenderOperation::RenderLineBreak,
        ]
    }
}

#[derive(Debug)]
struct IndexDrawer {
    prefix: Vec<RenderOperation>,
    titles: Vec<WeightedLine>,
    suffix: Vec<RenderOperation>,
    content_width: u16,
    state: PresentationState,
}

impl IndexDrawer {
    fn initialize_layout(&self, dimensions: &WindowSize, visible_titles: usize) -> Vec<RenderOperation> {
        let margin = dimensions.columns.saturating_sub(self.content_width) / 2;
        let properties = MarginProperties { horizontal_margin: Margin::Fixed(margin), bottom_slide_margin: 0 };
        // However many we see + 3 for the title and 1 at the bottom.
        let content_height = (visible_titles + 4) as u16;
        let target_row = dimensions.rows.saturating_sub(content_height) / 2;
        vec![RenderOperation::ApplyMargin(properties), RenderOperation::JumpToRow { index: target_row }]
    }
}

impl AsRenderOperations for IndexDrawer {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let current_slide_index = self.state.current_slide_index();
        let max_rows = (dimensions.rows as f64 * 0.8) as u16;
        let titles = self.titles.iter().cloned();
        let (skip, take) = match titles.len() as u16 > max_rows {
            true => {
                let start = (current_slide_index as u16).saturating_sub(max_rows / 2);
                let start = start.min(titles.len() as u16 - max_rows);
                (start as usize, max_rows as usize)
            }
            false => (0, titles.len()),
        };

        let visible_titles = self.titles.iter().cloned().enumerate().skip(skip).take(take);
        let mut operations = self.initialize_layout(dimensions, take);
        operations.extend(self.prefix.iter().cloned());
        for (index, mut title) in visible_titles {
            if index == current_slide_index {
                title.apply_style(TextStyle::default().bold());
            }
            let operation = RenderOperation::RenderText { line: title, alignment: Default::default() };
            operations.extend([operation, RenderOperation::RenderLineBreak]);
        }
        operations.extend(self.suffix.iter().cloned());
        operations
    }

    fn diffable_content(&self) -> Option<&str> {
        // The index is just a view over the underlying data so it won't change in isolation.
        None
    }
}
