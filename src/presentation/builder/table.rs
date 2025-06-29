use crate::{
    markdown::elements::{Line, Table, TableRow, Text},
    presentation::builder::{BuildResult, PresentationBuilder, error::BuildError},
    theme::ElementType,
};
use std::iter;

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_table(&mut self, table: Table) -> BuildResult {
        let widths: Vec<_> = (0..table.columns())
            .map(|column| table.iter_column(column).map(|text| text.width()).max().unwrap_or(0))
            .collect();
        let flattened_header = self.prepare_table_row(table.header, &widths)?;
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
            let flattened_row = self.prepare_table_row(row, &widths)?;
            self.push_text(flattened_row, ElementType::Table);
            self.push_line_break();
        }
        Ok(())
    }

    fn prepare_table_row(&self, row: TableRow, widths: &[usize]) -> Result<Line, BuildError> {
        let mut flattened_row = Line(Vec::new());
        for (column, text) in row.0.into_iter().enumerate() {
            let text = text.resolve(&self.theme.palette)?;
            if column > 0 {
                flattened_row.0.push(Text::from(" │ "));
            }
            let text_length = text.width();
            flattened_row.0.extend(text.0.into_iter());

            let cell_width = widths[column];
            if text_length < cell_width {
                let padding = " ".repeat(cell_width - text_length);
                flattened_row.0.push(Text::from(padding));
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
| Name   | Taste  |
| ------ | ------ |
| Potato | Great  |
| Carrot | Yuck   |
";
        let lines = Test::new(input).render().rows(6).columns(22).into_lines();
        let expected_lines = &[
            "                      ",
            "Name   │ Taste        ",
            "───────┼──────        ",
            "Potato │ Great        ",
            "Carrot │ Yuck         ",
            "                      ",
        ];
        assert_eq!(lines, expected_lines);
    }
}
