use crate::{
    markdown::elements::{Line, Table, TableColumnAlignment, Text},
    presentation::builder::{BuildResult, PresentationBuilder, error::BuildError},
    theme::{ElementType, raw::RawColor},
};
use std::iter;

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_table(&mut self, table: Table) -> BuildResult {
        let widths: Vec<_> = (0..table.columns())
            .map(|column| table.iter_column(column).map(|text| text.width()).max().unwrap_or(0))
            .collect();
        let incremental = self.slide_state.incremental_tables.unwrap_or(self.options.incremental_tables);
        let alignments: Vec<_> = table.columns.iter().map(|c| c.alignment).collect();
        let column_texts = table.columns.into_iter().map(|c| c.text);
        let flattened_header =
            self.prepare_table_row(column_texts, iter::repeat(TableColumnAlignment::Center), &widths)?;
        if incremental && self.options.pause_before_incremental_tables {
            self.push_pause();
        }
        self.push_text(flattened_header, ElementType::Table);
        self.push_line_break();

        let mut separator = Line(Vec::new());
        for (index, width) in widths.iter().enumerate() {
            let mut contents = String::new();
            let mut margin = 1;
            if index > 0 {
                contents.push('┼');
                // Append an extra dash to have 1 column margin on both sides
                if index < widths.len() - 1 {
                    margin += 1;
                }
            }
            contents.extend(iter::repeat_n("─", *width + margin));
            separator.0.push(Text::from(contents));
        }

        self.push_text(separator, ElementType::Table);
        self.push_line_break();

        for row in table.rows {
            if incremental {
                self.push_pause();
            }
            let flattened_row = self.prepare_table_row(row.0, alignments.iter().copied(), &widths)?;
            self.push_text(flattened_row, ElementType::Table);
            self.push_line_break();
        }
        if incremental && self.options.pause_after_incremental_tables {
            self.push_pause();
        }
        Ok(())
    }

    fn prepare_table_row<I, A>(&self, texts: I, alignments: A, widths: &[usize]) -> Result<Line, BuildError>
    where
        I: IntoIterator<Item = Line<RawColor>>,
        A: IntoIterator<Item = TableColumnAlignment>,
    {
        let mut flattened_row = Line(Vec::new());
        for (column, (text, alignment)) in texts.into_iter().zip(alignments).enumerate() {
            let text = text.resolve(&self.theme.palette)?;
            if column > 0 {
                flattened_row.0.push(Text::from(" │ "));
            }
            let text_length = text.width();
            let cell_width = widths[column];
            let padding = cell_width.saturating_sub(text_length);
            if padding == 0 {
                flattened_row.0.extend(text.0.into_iter());
            } else {
                match alignment {
                    TableColumnAlignment::Left => {
                        flattened_row.0.extend(text.0.into_iter());
                        flattened_row.0.push(Text::from(" ".repeat(padding)));
                    }
                    TableColumnAlignment::Center => {
                        let padding_after = padding / 2;
                        let padding_before = padding_after + (padding % 2);
                        flattened_row.0.push(Text::from(" ".repeat(padding_before)));
                        flattened_row.0.extend(text.0.into_iter());
                        flattened_row.0.push(Text::from(" ".repeat(padding_after)));
                    }
                    TableColumnAlignment::Right => {
                        let padding = " ".repeat(padding);
                        flattened_row.0.push(Text::from(padding));
                        flattened_row.0.extend(text.0.into_iter());
                    }
                }
            }
        }
        Ok(flattened_row)
    }
}

#[cfg(test)]
mod tests {
    use crate::presentation::builder::utils::Test;

    #[test]
    fn table() {
        let input = "
| Name       | Taste  |
| ---------- | ------ |
| Potatooo   | Great  |
| Carrot     | Yuck   |
";
        let lines = Test::new(input).render().rows(6).columns(18).into_lines();
        let expected_lines = &[
            "                  ",
            "  Name   │ Taste  ",
            "─────────┼──────  ",
            "Potatooo │ Great  ",
            "Carrot   │ Yuck   ",
            "                  ",
        ];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn table_left_aligned() {
        let input = "
| Name       | Taste  |
| :--------- | ------ |
| Potatooo   | Great  |
| Carrot     | Yuck   |
";
        let lines = Test::new(input).render().rows(6).columns(18).into_lines();
        let expected_lines = &[
            "                  ",
            "  Name   │ Taste  ",
            "─────────┼──────  ",
            "Potatooo │ Great  ",
            "Carrot   │ Yuck   ",
            "                  ",
        ];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn table_center_aligned() {
        let input = "
| Name       | Taste  |
| :--------: | ------ |
| Potatooo   | Great  |
| Carrot     | Yuck   |
";
        let lines = Test::new(input).render().rows(6).columns(18).into_lines();
        let expected_lines = &[
            "                  ",
            "  Name   │ Taste  ",
            "─────────┼──────  ",
            "Potatooo │ Great  ",
            " Carrot  │ Yuck   ",
            "                  ",
        ];
        assert_eq!(lines, expected_lines);
    }

    #[test]
    fn table_right_aligned() {
        let input = "
| Name       | Taste  |
| ---------: | ------ |
| Potatooo   | Great  |
| Carrot     | Yuck   |
";
        let lines = Test::new(input).render().rows(6).columns(18).into_lines();
        let expected_lines = &[
            "                  ",
            "  Name   │ Taste  ",
            "─────────┼──────  ",
            "Potatooo │ Great  ",
            "  Carrot │ Yuck   ",
            "                  ",
        ];
        assert_eq!(lines, expected_lines);
    }
}
