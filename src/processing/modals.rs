use crate::{
    markdown::{
        elements::{Text, TextBlock},
        text::{WeightedText, WeightedTextBlock},
    },
    presentation::{AsRenderOperations, MarginProperties, PresentationState, RenderOperation},
    processing::padding::NumberPadder,
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::Margin,
    PresentationTheme,
};
use std::{iter, rc::Rc};

#[derive(Default)]
pub(crate) struct IndexBuilder {
    titles: Vec<TextBlock>,
}

impl IndexBuilder {
    pub(crate) fn add_title(&mut self, title: TextBlock) {
        self.titles.push(title);
    }

    pub(crate) fn build(self, theme: &PresentationTheme, state: PresentationState) -> Vec<RenderOperation> {
        let mut builder = ModalBuilder::new("Slides");
        let padder = NumberPadder::new(self.titles.len());
        for (index, mut title) in self.titles.into_iter().enumerate() {
            let index = padder.pad_right(index + 1);
            title.chunks.insert(0, format!("{index}: ").into());
            builder.content.push(title);
        }
        let base_color = theme.modals.colors.merge(&theme.default_style.colors);
        let selection_style = TextStyle::default().colors(theme.modals.selection_colors.clone()).bold();
        let ModalContent { prefix, content, suffix, content_width } = builder.build(base_color);
        let drawer = IndexDrawer { prefix, rows: content, suffix, state, content_width, selection_style };
        vec![RenderOperation::RenderDynamic(Rc::new(drawer))]
    }
}

#[derive(Debug)]
struct IndexDrawer {
    prefix: Vec<RenderOperation>,
    rows: Vec<ContentRow>,
    suffix: Vec<RenderOperation>,
    content_width: u16,
    state: PresentationState,
    selection_style: TextStyle,
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
        let (skip, take) = match self.rows.len() as u16 > max_rows {
            true => {
                let start = (current_slide_index as u16).saturating_sub(max_rows / 2);
                let start = start.min(self.rows.len() as u16 - max_rows);
                (start as usize, max_rows as usize)
            }
            false => (0, self.rows.len()),
        };
        let visible_rows = self.rows.iter().enumerate().skip(skip).take(take);
        let mut operations = self.initialize_layout(dimensions, take);
        operations.extend(self.prefix.iter().cloned());
        for (index, row) in visible_rows {
            let mut row = row.clone();
            if index == current_slide_index {
                row = row.with_style(self.selection_style.clone());
            }
            let operation = RenderOperation::RenderText { line: row.build(), alignment: Default::default() };
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
    content: Vec<TextBlock>,
}

impl ModalBuilder {
    fn new<S: Into<String>>(heading: S) -> Self {
        Self { heading: heading.into(), content: Vec::new() }
    }

    fn build(self, colors: Colors) -> ModalContent {
        let longest_line = self.content.iter().map(TextBlock::width).max().unwrap_or(0) as u16;
        let longest_line = longest_line.max(self.heading.len() as u16);
        // Ensure we have a minimum width so it doesn't look too narrow.
        let longest_line = longest_line.max(12);
        // The final text looks like "|  <content>  |"
        let content_width = longest_line + 6;
        let mut prefix = vec![RenderOperation::SetColors(colors)];

        let heading = Self::center_line(self.heading, longest_line as usize);
        prefix.extend(Border::Top.render_line(content_width));
        prefix.extend([
            RenderOperation::RenderText {
                line: Self::build_line([Text::from(heading)], content_width).build(),
                alignment: Default::default(),
            },
            RenderOperation::RenderLineBreak,
        ]);
        prefix.extend(Border::Separator.render_line(content_width));
        let mut content = Vec::new();
        for title in self.content {
            content.push(Self::build_line(title.chunks, content_width));
        }
        let suffix = Border::Bottom.render_line(content_width).into_iter().collect();
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

    fn build_line<C>(text_chunks: C, content_width: u16) -> ContentRow
    where
        C: IntoIterator<Item = Text>,
    {
        let (opening, closing) = Border::Regular.edges();
        let prefix = WeightedText::from(format!("{opening}  "));
        let content: Vec<_> = text_chunks.into_iter().map(WeightedText::from).collect();
        let total_width = content.iter().map(|c| c.width()).sum::<usize>() + prefix.width();
        let missing = content_width as usize - 1 - total_width;

        let mut suffix = " ".repeat(missing);
        suffix.push(closing);
        let suffix = WeightedText::from(suffix);
        ContentRow { prefix, content, suffix }
    }
}

struct ModalContent {
    prefix: Vec<RenderOperation>,
    content: Vec<ContentRow>,
    suffix: Vec<RenderOperation>,
    content_width: u16,
}

#[derive(Clone, Debug)]
struct ContentRow {
    prefix: WeightedText,
    content: Vec<WeightedText>,
    suffix: WeightedText,
}

impl ContentRow {
    fn with_style(mut self, style: TextStyle) -> ContentRow {
        for chunk in &mut self.content {
            chunk.style_mut().merge(&style);
        }
        self
    }

    fn build(self) -> WeightedTextBlock {
        let mut chunks = self.content;
        chunks.insert(0, self.prefix);
        chunks.push(self.suffix);
        WeightedTextBlock::from(chunks)
    }
}

enum Border {
    Regular,
    Top,
    Separator,
    Bottom,
}

impl Border {
    fn render_line(&self, content_length: u16) -> [RenderOperation; 2] {
        let (opening, closing) = self.edges();
        let mut line = String::from(opening);
        line.push_str(&"─".repeat(content_length.saturating_sub(2) as usize));
        line.push(closing);
        let horizontal_border = WeightedTextBlock::from(vec![WeightedText::from(line)]);
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
