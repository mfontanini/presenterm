use crate::theme::Alignment;
use crossterm::terminal::WindowSize;

pub(crate) struct FixedLayout<'a>(pub(crate) &'a Alignment);

impl<'a> FixedLayout<'a> {
    pub(crate) fn start_column(&self, dimensions: &WindowSize, line_length: u16) -> u16 {
        match *self.0 {
            Alignment::Left { margin } => margin,
            Alignment::Center { minimum_margin, minimum_size } => {
                let max_line_length = line_length.max(minimum_size);
                let column = (dimensions.columns - max_line_length) / 2;
                column.max(minimum_margin)
            }
        }
    }
}

pub(crate) struct WordWrapLayout<'a>(pub(crate) &'a Alignment);

impl<'a> WordWrapLayout<'a> {
    pub(crate) fn compute(&self, dimensions: &WindowSize, text_length: u16) -> TextPositioning {
        let mut line_length = dimensions.columns;
        let mut start_column;
        match *self.0 {
            Alignment::Left { margin } => {
                start_column = margin;
                line_length -= margin * 2;
            }
            Alignment::Center { minimum_margin, minimum_size } => {
                line_length = text_length.min(dimensions.columns - minimum_margin * 2).max(minimum_size);
                if line_length > dimensions.columns {
                    start_column = minimum_margin;
                } else {
                    start_column = (dimensions.columns - line_length) / 2;
                    start_column = start_column.max(minimum_margin);
                }
            }
        };
        TextPositioning { line_length, start_column }
    }
}

pub(crate) struct TextPositioning {
    pub(crate) line_length: u16,
    pub(crate) start_column: u16,
}
