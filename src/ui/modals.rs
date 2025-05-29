use crate::{
    code::padding::NumberPadder,
    commands::keyboard::KeyBinding,
    config::KeyBindingsConfig,
    markdown::{
        elements::{Line, Text},
        text::WeightedLine,
        text_style::TextStyle,
    },
    presentation::PresentationState,
    render::{
        operation::{
            AsRenderOperations, ImagePosition, ImageRenderProperties, ImageSize, MarginProperties, RenderOperation,
        },
        properties::WindowSize,
    },
    terminal::image::Image,
    theme::{Margin, PresentationTheme},
};
use std::{iter, rc::Rc};
use unicode_width::UnicodeWidthStr;

static MODAL_Z_INDEX: i32 = -1;

#[derive(Default)]
pub(crate) struct IndexBuilder {
    titles: Vec<Line>,
    background: Option<Image>,
}

impl IndexBuilder {
    pub(crate) fn add_title(&mut self, title: Line) {
        self.titles.push(title);
    }

    pub(crate) fn set_background(&mut self, background: Image) {
        self.background = Some(background);
    }

    pub(crate) fn build(self, theme: &PresentationTheme, state: PresentationState) -> Vec<RenderOperation> {
        let mut builder = ModalBuilder::new("Slides");
        let padder = NumberPadder::new(self.titles.len());
        for (index, mut title) in self.titles.into_iter().enumerate() {
            let index = padder.pad_right(index + 1);
            title.0.insert(0, format!("{index}: ").into());
            builder.content.push(title);
        }
        let base_style = theme.modals.style;
        let selection_style = theme.modals.selection_style;
        let ModalContent { prefix, content, suffix, content_width } = builder.build(base_style);
        let drawer = IndexDrawer {
            prefix,
            rows: content,
            suffix,
            state,
            content_width,
            selection_style,
            background: self.background,
        };
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
    background: Option<Image>,
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
        let mut operations = vec![CenterModalContent::new(self.content_width, take, self.background.clone()).into()];
        operations.extend(self.prefix.iter().cloned());
        for (index, row) in visible_rows {
            let mut row = row.clone();
            if index == current_slide_index {
                row = row.with_style(self.selection_style);
            }
            let operation = RenderOperation::RenderText { line: row.build(), properties: Default::default() };
            operations.extend([operation, RenderOperation::RenderLineBreak]);
        }
        operations.extend(self.suffix.iter().cloned());
        operations
    }
}

#[derive(Default)]
pub(crate) struct KeyBindingsModalBuilder {
    background: Option<Image>,
}

impl KeyBindingsModalBuilder {
    pub(crate) fn set_background(&mut self, background: Image) {
        self.background = Some(background);
    }

    pub(crate) fn build(self, theme: &PresentationTheme, config: &KeyBindingsConfig) -> Vec<RenderOperation> {
        let mut builder = ModalBuilder::new("Key bindings");
        builder.content.extend([
            Self::build_line("Next", &config.next),
            Self::build_line("Next (fast)", &config.next_fast),
            Self::build_line("Previous", &config.previous),
            Self::build_line("Previous (fast)", &config.previous_fast),
            Self::build_line("First slide", &config.first_slide),
            Self::build_line("Last slide", &config.last_slide),
            Self::build_line("Go to slide", &config.go_to_slide),
            Self::build_line("Execute code", &config.execute_code),
            Self::build_line("Reload", &config.reload),
            Self::build_line("Toggle slide index", &config.toggle_slide_index),
            Self::build_line("Close modal", &config.close_modal),
            Self::build_line("Exit", &config.exit),
        ]);
        let lines = builder.content.len();
        let style = theme.modals.style;
        let content = builder.build(style);
        let content_width = content.content_width;
        let mut operations = content.into_operations();
        operations.insert(0, CenterModalContent::new(content_width, lines, self.background).into());
        operations
    }

    fn build_line(label: &str, bindings: &[KeyBinding]) -> Line {
        let mut text = vec![Text::new(label, TextStyle::default().bold()), ": ".into()];
        for (index, binding) in bindings.iter().enumerate() {
            if index > 0 {
                text.push(", ".into());
            }
            text.push(Text::new(binding.to_string(), TextStyle::default().italics()));
        }
        Line(text)
    }
}

struct ModalBuilder {
    heading: String,
    content: Vec<Line>,
}

impl ModalBuilder {
    fn new<S: Into<String>>(heading: S) -> Self {
        Self { heading: heading.into(), content: Vec::new() }
    }

    fn build(self, style: TextStyle) -> ModalContent {
        let longest_line = self.content.iter().map(Line::width).max().unwrap_or(0) as u16;
        let longest_line = longest_line.max(self.heading.len() as u16);
        // Ensure we have a minimum width so it doesn't look too narrow.
        let longest_line = longest_line.max(12);
        // The final text looks like "|  <content>  |"
        let content_width = longest_line + 6;
        let mut prefix = vec![RenderOperation::SetColors(style.colors)];

        let heading = Self::center_line(self.heading, longest_line as usize);
        prefix.extend(Border::Top.render_line(content_width));
        prefix.extend([
            RenderOperation::RenderText {
                line: Self::build_line(vec![Text::from(heading)], content_width).build(),
                properties: Default::default(),
            },
            RenderOperation::RenderLineBreak,
        ]);
        prefix.extend(Border::Separator.render_line(content_width));
        let mut content = Vec::new();
        for title in self.content {
            content.push(Self::build_line(title.0, content_width));
        }
        let suffix = Border::Bottom.render_line(content_width).into_iter().collect();
        ModalContent { prefix, content, suffix, content_width }
    }

    fn center_line(text: String, longest_line: usize) -> String {
        let missing = longest_line.saturating_sub(text.len());
        let padding = missing / 2;
        let mut output = " ".repeat(padding);
        output.push_str(&text);
        output.extend(iter::repeat_n(' ', padding));
        output
    }

    fn build_line(text_chunks: Vec<Text>, content_width: u16) -> ContentRow {
        let (opening, closing) = Border::Regular.edges();
        let prefix = Text::from(format!("{opening}  "));
        let content = text_chunks;
        let total_width = content.iter().map(|c| c.content.width()).sum::<usize>() + prefix.content.width();
        let missing = content_width as usize - 1 - total_width;

        let mut suffix = " ".repeat(missing);
        suffix.push(closing);
        ContentRow { prefix, content, suffix: suffix.into() }
    }
}

struct ModalContent {
    prefix: Vec<RenderOperation>,
    content: Vec<ContentRow>,
    suffix: Vec<RenderOperation>,
    content_width: u16,
}

impl ModalContent {
    fn into_operations(self) -> Vec<RenderOperation> {
        let mut operations = self.prefix;
        operations.extend(self.content.into_iter().flat_map(|c| {
            [
                RenderOperation::RenderText { line: c.build(), properties: Default::default() },
                RenderOperation::RenderLineBreak,
            ]
        }));
        operations.extend(self.suffix);
        operations
    }
}

#[derive(Clone, Debug)]
struct ContentRow {
    prefix: Text,
    content: Vec<Text>,
    suffix: Text,
}

impl ContentRow {
    fn with_style(mut self, style: TextStyle) -> ContentRow {
        for chunk in &mut self.content {
            chunk.style.merge(&style);
        }
        self
    }

    fn build(self) -> WeightedLine {
        let mut chunks = self.content;
        chunks.insert(0, self.prefix);
        chunks.push(self.suffix);
        WeightedLine::from(chunks)
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
        let horizontal_border = WeightedLine::from(vec![Text::from(line)]);
        [
            RenderOperation::RenderText { line: horizontal_border.clone(), properties: Default::default() },
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

#[derive(Debug)]
struct CenterModalContent {
    content_width: u16,
    content_height: usize,
    background: Option<Image>,
}

impl CenterModalContent {
    fn new(content_width: u16, content_height: usize, background: Option<Image>) -> Self {
        Self { content_width, content_height, background }
    }
}

impl AsRenderOperations for CenterModalContent {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let margin = dimensions.columns.saturating_sub(self.content_width) / 2;
        let properties = MarginProperties { horizontal: Margin::Fixed(margin), top: 0, bottom: 0 };
        // However many we see + 3 for the title and 1 at the bottom.
        let content_height = (self.content_height + 4) as u16;
        let target_row = dimensions.rows.saturating_sub(content_height) / 2;

        let mut operations =
            vec![RenderOperation::ApplyMargin(properties), RenderOperation::JumpToRow { index: target_row }];
        if let Some(image) = &self.background {
            let properties = ImageRenderProperties {
                z_index: MODAL_Z_INDEX,
                size: ImageSize::Specific(self.content_width, content_height),
                restore_cursor: true,
                background_color: None,
                position: ImagePosition::Center,
            };
            operations.push(RenderOperation::RenderImage(image.clone(), properties));
        }
        operations
    }
}

impl From<CenterModalContent> for RenderOperation {
    fn from(op: CenterModalContent) -> Self {
        Self::RenderDynamic(Rc::new(op))
    }
}
