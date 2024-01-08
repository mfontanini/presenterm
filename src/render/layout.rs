use crate::{render::properties::WindowSize, theme::Alignment};

#[derive(Debug)]
pub(crate) struct Layout {
    alignment: Alignment,
    start_column_offset: u16,
}

impl Layout {
    pub(crate) fn new(alignment: Alignment) -> Self {
        Self { alignment, start_column_offset: 0 }
    }

    pub(crate) fn with_start_column(mut self, column: u16) -> Self {
        self.start_column_offset = column;
        self
    }

    pub(crate) fn compute(&self, dimensions: &WindowSize, text_length: u16) -> Positioning {
        let max_line_length;
        let mut start_column;
        match &self.alignment {
            Alignment::Left { margin } => {
                let margin = margin.as_characters(dimensions.columns);
                // Ignore the margin if it's larger than the screen: we can't satisfy it so we
                // might as well not do anything about it.
                let margin = Self::fit_to_columns(dimensions, margin.saturating_mul(2), margin);
                start_column = margin;
                max_line_length = dimensions.columns - margin.saturating_mul(2);
            }
            Alignment::Right { margin } => {
                let margin = margin.as_characters(dimensions.columns);
                let margin = Self::fit_to_columns(dimensions, margin.saturating_mul(2), margin);
                start_column = dimensions.columns.saturating_sub(margin).saturating_sub(text_length).max(margin);
                max_line_length = (dimensions.columns - margin) - start_column;
            }
            Alignment::Center { minimum_margin, minimum_size } => {
                let minimum_margin = minimum_margin.as_characters(dimensions.columns);
                // Respect minimum size as much as we can if both together overflow.
                let minimum_size = dimensions.columns.min(*minimum_size);
                let minimum_margin = Self::fit_to_columns(
                    dimensions,
                    minimum_margin.saturating_mul(2).saturating_add(minimum_size),
                    minimum_margin,
                );
                max_line_length =
                    text_length.min(dimensions.columns - minimum_margin.saturating_mul(2)).max(minimum_size);
                if max_line_length > dimensions.columns {
                    start_column = minimum_margin;
                } else {
                    start_column = (dimensions.columns - max_line_length) / 2;
                    start_column = start_column.max(minimum_margin);
                }
            }
        };
        start_column += self.start_column_offset;
        Positioning { max_line_length, start_column }
    }

    fn fit_to_columns(dimensions: &WindowSize, required_fit: u16, actual_fit: u16) -> u16 {
        if required_fit > dimensions.columns { 0 } else { actual_fit }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Positioning {
    pub(crate) max_line_length: u16,
    pub(crate) start_column: u16,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::theme::Margin;
    use rstest::rstest;

    #[rstest]
    #[case::left_no_margin(
        Alignment::Left{ margin: Margin::Fixed(0) },
        10,
        Positioning{ max_line_length: 100, start_column: 0 }
    )]
    #[case::left_some_margin(
        Alignment::Left{ margin: Margin::Fixed(5) },
        10,
        Positioning{ max_line_length: 90, start_column: 5 }
    )]
    #[case::left_line_overflows(
        Alignment::Left{ margin: Margin::Fixed(5) },
        150,
        Positioning{ max_line_length: 90, start_column: 5 }
    )]
    #[case::left_large_margin(
        Alignment::Left{ margin: Margin::Fixed(60) },
        10,
        Positioning{ max_line_length: 100, start_column: 0 }
    )]
    #[case::left_margin_too_large(
        Alignment::Left{ margin: Margin::Fixed(105) },
        10,
        Positioning{ max_line_length: 100, start_column: 0 }
    )]
    #[case::right_no_margin(
        Alignment::Right{ margin: Margin::Fixed(0) },
        10,
        Positioning{ max_line_length: 10, start_column: 90 }
    )]
    #[case::right_some_margin(
        Alignment::Right{ margin: Margin::Fixed(5) },
        10,
        Positioning{ max_line_length: 10, start_column: 85 }
    )]
    #[case::right_line_overflows(
        Alignment::Right{ margin: Margin::Fixed(5) },
        150,
        Positioning{ max_line_length: 90, start_column: 5 }
    )]
    #[case::right_large_margin(
        Alignment::Right{ margin: Margin::Fixed(60) },
        10,
        Positioning{ max_line_length: 10, start_column: 90 }
    )]
    #[case::right_margin_too_large(
        Alignment::Right{ margin: Margin::Fixed(105) },
        10,
        Positioning{ max_line_length: 10, start_column: 90 }
    )]
    #[case::center_no_minimums(
        Alignment::Center{ minimum_margin: Margin::Fixed(0), minimum_size: 0 },
        10,
        Positioning{ max_line_length: 10, start_column: 45 }
    )]
    #[case::center_minimum_margin(
        Alignment::Center{ minimum_margin: Margin::Fixed(10), minimum_size: 0 },
        100,
        Positioning{ max_line_length: 80, start_column: 10 }
    )]
    #[case::center_minimum_size(
        Alignment::Center{ minimum_margin: Margin::Fixed(0), minimum_size: 50 },
        10,
        Positioning{ max_line_length: 50, start_column: 25 }
    )]
    #[case::center_large_minimum_margin(
        Alignment::Center{ minimum_margin: Margin::Fixed(60), minimum_size: 0 },
        10,
        Positioning{ max_line_length: 10, start_column: 45 }
    )]
    #[case::center_minimum_margin_too_large(
        Alignment::Center{ minimum_margin: Margin::Fixed(105), minimum_size: 0 },
        10,
        Positioning{ max_line_length: 10, start_column: 45 }
    )]
    #[case::center_minimum_size_too_large(
        Alignment::Center{ minimum_margin: Margin::Fixed(0), minimum_size: 105 },
        10,
        Positioning{ max_line_length: 100, start_column: 0 }
    )]
    #[case::center_margin_and_size_overflows(
        Alignment::Center{ minimum_margin: Margin::Fixed(30), minimum_size: 60 },
        10,
        Positioning{ max_line_length: 60, start_column: 20 }
    )]
    fn layout(#[case] alignment: Alignment, #[case] length: u16, #[case] expected: Positioning) {
        let dimensions = WindowSize { rows: 0, columns: 100, width: 0, height: 0 };
        let positioning = Layout::new(alignment).compute(&dimensions, length);
        assert_eq!(positioning, expected);
    }
}
