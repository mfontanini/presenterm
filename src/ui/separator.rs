use crate::{
    markdown::{
        elements::{Line, Text},
        text_style::TextStyle,
    },
    render::{
        layout::{Layout, Positioning},
        operation::{AsRenderOperations, BlockLine, RenderOperation},
        properties::WindowSize,
    },
    theme::{Margin, clean::Alignment},
};
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub(crate) enum SeparatorWidth {
    Fixed(u16),

    #[default]
    FitToWindow,
}

#[derive(Clone, Debug)]
pub(crate) struct RenderSeparator {
    heading: Line,
    width: SeparatorWidth,
    font_size: u8,
}

impl RenderSeparator {
    pub(crate) fn new<S: Into<Line>>(heading: S, width: SeparatorWidth, font_size: u8) -> Self {
        let mut heading: Line = heading.into();
        heading.apply_style(&TextStyle::default().size(font_size));
        Self { heading, width, font_size }
    }
}

impl From<RenderSeparator> for RenderOperation {
    fn from(separator: RenderSeparator) -> Self {
        Self::RenderDynamic(Rc::new(separator))
    }
}

impl AsRenderOperations for RenderSeparator {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let character = "—";
        let width = match self.width {
            SeparatorWidth::Fixed(width) => {
                let Positioning { max_line_length, .. } =
                    Layout::new(Alignment::Center { minimum_margin: Margin::Fixed(0), minimum_size: 0 })
                        .with_font_size(self.font_size)
                        .compute(dimensions, width);
                max_line_length.min(width) as usize
            }
            SeparatorWidth::FitToWindow => dimensions.columns as usize,
        };
        let style = TextStyle::default().size(self.font_size);
        let separator = match self.heading.width() == 0 {
            true => Line::from(Text::new(character.repeat(width / self.font_size as usize), style)),
            false => {
                let width = width.saturating_sub(self.heading.width());
                let (dashes_len, remainder) = (width / 2, width % 2);
                let mut dashes = character.repeat(dashes_len);
                let mut line = Line::from(Text::new(dashes.clone(), style));
                line.0.extend(self.heading.0.iter().cloned());

                if remainder > 0 {
                    dashes.push_str(character);
                }
                line.0.push(Text::new(dashes, style));
                line
            }
        };
        vec![RenderOperation::RenderBlockLine(BlockLine {
            prefix: "".into(),
            right_padding_length: 0,
            repeat_prefix_on_wrap: false,
            text: separator.into(),
            block_length: width as u16,
            block_color: None,
            alignment: Alignment::Center { minimum_size: 1, minimum_margin: Margin::Fixed(0) },
        })]
    }
}
