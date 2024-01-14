use crate::{
    markdown::{
        elements::{StyledText, Text},
        text::{WeightedLine, WeightedText},
    },
    presentation::{AsRenderOperations, MarginProperties, PresentationState, RenderOperation},
    processing::padding::NumberPadder,
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::Margin,
};
use std::{iter, rc::Rc};

#[derive(Default)]
pub(crate) struct IndexBuilder {
    titles: Vec<Text>,
}

impl IndexBuilder {
    pub(crate) fn add_title(&mut self, title: Text) {
        self.titles.push(title);
    }

    pub(crate) fn build(self, colors: Colors, state: PresentationState) -> Vec<RenderOperation> {
        let mut builder = ModalBuilder::new("Slides");
        let padder = NumberPadder::new(self.titles.len());
        for (index, mut title) in self.titles.into_iter().enumerate() {
            let index = padder.pad_right(index + 1);
            title.chunks.insert(0, format!("{index}: ").into());
            builder.content.push(title);
        }
        let ModalContent { prefix, content, suffix, content_width } = builder.build(colors);
        let drawer = IndexDrawer { prefix, titles: content, suffix, state, content_width };
        vec![RenderOperation::RenderDynamic(Rc::new(drawer))]
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

struct ModalBuilder {
    heading: String,
    content: Vec<Text>,
}

impl ModalBuilder {
    fn new<S: Into<String>>(heading: S) -> Self {
        Self { heading: heading.into(), content: Vec::new() }
    }

    fn build(self, colors: Colors) -> ModalContent {
        let longest_line = self.content.iter().map(Text::width).max().unwrap_or(0) as u16;
        let longest_line = longest_line.max(self.heading.len() as u16);
        // Ensure we have a minimum width so it doesn't look too narrow.
        let longest_line = longest_line.max(12);
        // The final text looks like "|  <content>  |"
        let content_width = longest_line + 6;
        let mut prefix = vec![RenderOperation::SetColors(colors)];

        let heading = Self::center_line(self.heading, longest_line as usize);
        prefix.extend(ModalRow::Top.render_line(content_width));
        prefix.extend([
            RenderOperation::RenderText {
                line: Self::build_line([StyledText::from(heading)], content_width),
                alignment: Default::default(),
            },
            RenderOperation::RenderLineBreak,
        ]);
        prefix.extend(ModalRow::Separator.render_line(content_width));
        let mut content = Vec::new();
        for title in self.content {
            content.push(Self::build_line(title.chunks, content_width));
        }
        let suffix = ModalRow::Bottom.render_line(content_width).into_iter().collect();
        ModalContent { prefix, content, suffix, content_width }
    }

    fn center_line(text: String, longest_line: usize) -> String {
        let missing = longest_line.saturating_sub(text.len());
        let padding = missing / 2;
        let mut output = " ".repeat(padding);
        output.push_str(&text);
        output.extend(iter::repeat(' ').take(padding));
        output
    }

    fn build_line<C>(text_chunks: C, content_width: u16) -> WeightedLine
    where
        C: IntoIterator<Item = StyledText>,
    {
        let (opening, closing) = ModalRow::Regular.edges();
        let mut chunks = vec![WeightedText::from(format!("{opening}  "))];
        chunks.extend(text_chunks.into_iter().map(WeightedText::from));
        let missing = content_width as usize - 1 - chunks.iter().map(|c| c.width()).sum::<usize>();
        chunks.extend([WeightedText::from(" ".repeat(missing)), WeightedText::from(closing.to_string())]);

        WeightedLine::from(chunks)
    }
}

struct ModalContent {
    prefix: Vec<RenderOperation>,
    content: Vec<WeightedLine>,
    suffix: Vec<RenderOperation>,
    content_width: u16,
}

enum ModalRow {
    Regular,
    Top,
    Separator,
    Bottom,
}

impl ModalRow {
    fn render_line(&self, content_length: u16) -> [RenderOperation; 2] {
        let (opening, closing) = self.edges();
        let mut line = String::from(opening);
        line.push_str(&"─".repeat(content_length.saturating_sub(2) as usize));
        line.push(closing);
        let horizontal_border = WeightedLine::from(vec![WeightedText::from(line)]);
        [
            RenderOperation::RenderText { line: horizontal_border.clone(), alignment: Default::default() },
            RenderOperation::RenderLineBreak,
        ]
    }

    fn edges(&self) -> (char, char) {
        match self {
            Self::Regular => ('│', '│'),
            Self::Top => ('┌', '┐'),
            Self::Separator => ('├', '┤'),
            Self::Bottom => ('└', '┘'),
        }
    }
}
