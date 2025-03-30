use super::{
    image::{
        Image,
        printer::{PrintImageError, PrintOptions},
    },
    printer::{TerminalError, TerminalIo},
};
use crate::{
    WindowSize,
    markdown::{
        elements::Text,
        text_style::{Color, Colors, TextStyle},
    },
    terminal::printer::TerminalCommand,
};
use std::{collections::HashMap, io};

#[derive(Debug)]
pub(crate) struct PrintedImage {
    pub(crate) image: Image,
    pub(crate) width_columns: u16,
}

pub(crate) struct TerminalRowIterator<'a> {
    row: &'a [StyledChar],
}

impl<'a> TerminalRowIterator<'a> {
    pub(crate) fn new(row: &'a [StyledChar]) -> Self {
        Self { row }
    }
}

impl Iterator for TerminalRowIterator<'_> {
    type Item = Text;

    fn next(&mut self) -> Option<Self::Item> {
        let style = self.row.first()?.style;
        let mut output = String::new();
        while let Some(c) = self.row.first() {
            if c.style != style {
                break;
            }
            output.push(c.character);
            self.row = &self.row[1..];
        }
        Some(Text::new(output, style))
    }
}

#[derive(Debug)]
pub(crate) struct TerminalGrid {
    pub(crate) rows: Vec<Vec<StyledChar>>,
    pub(crate) background_color: Option<Color>,
    pub(crate) images: HashMap<(u16, u16), PrintedImage>,
}

pub(crate) struct VirtualTerminal {
    row: u16,
    column: u16,
    colors: Colors,
    rows: Vec<Vec<StyledChar>>,
    background_color: Option<Color>,
    images: HashMap<(u16, u16), PrintedImage>,
    row_heights: Vec<u16>,
}

impl VirtualTerminal {
    pub(crate) fn new(dimensions: WindowSize) -> Self {
        let rows = vec![vec![StyledChar::default(); dimensions.columns as usize]; dimensions.rows as usize];
        let row_heights = vec![1; dimensions.rows as usize];
        Self {
            row: 0,
            column: 0,
            colors: Default::default(),
            rows,
            background_color: None,
            images: Default::default(),
            row_heights,
        }
    }

    pub(crate) fn into_contents(self) -> TerminalGrid {
        TerminalGrid { rows: self.rows, background_color: self.background_color, images: self.images }
    }

    fn current_cell_mut(&mut self) -> Option<&mut StyledChar> {
        self.rows.get_mut(self.row as usize).and_then(|row| row.get_mut(self.column as usize))
    }

    fn set_current_row_height(&mut self, height: u16) {
        if let Some(current) = self.row_heights.get_mut(self.row as usize) {
            *current = height;
        }
    }

    fn current_row_height(&self) -> u16 {
        *self.row_heights.get(self.row as usize).unwrap_or(&1)
    }

    fn move_to(&mut self, column: u16, row: u16) -> io::Result<()> {
        self.column = column;
        self.row = row;
        Ok(())
    }

    fn move_to_row(&mut self, row: u16) -> io::Result<()> {
        self.row = row;
        self.set_current_row_height(1);
        Ok(())
    }

    fn move_to_column(&mut self, column: u16) -> io::Result<()> {
        self.column = column;
        Ok(())
    }

    fn move_down(&mut self, amount: u16) -> io::Result<()> {
        self.row += amount;
        Ok(())
    }

    fn move_right(&mut self, amount: u16) -> io::Result<()> {
        self.column += amount;
        Ok(())
    }

    fn move_to_next_line(&mut self) -> io::Result<()> {
        let amount = self.current_row_height();
        self.row += amount;
        self.column = 0;
        self.set_current_row_height(1);
        Ok(())
    }

    fn print_text(&mut self, content: &str, style: &TextStyle) -> io::Result<()> {
        let style = style.merged(&TextStyle::default().colors(self.colors));
        for c in content.chars() {
            let Some(cell) = self.current_cell_mut() else {
                continue;
            };
            cell.character = c;
            cell.style = style;
            self.column += 1;
        }
        let height = self.current_row_height().max(style.size as u16);
        self.set_current_row_height(height);
        Ok(())
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        for row in &mut self.rows {
            for cell in row {
                cell.character = ' ';
            }
        }
        self.background_color = self.colors.background;
        Ok(())
    }

    fn set_colors(&mut self, colors: crate::markdown::text_style::Colors) -> io::Result<()> {
        self.colors = colors;
        Ok(())
    }

    fn set_background_color(&mut self, color: Color) -> io::Result<()> {
        self.colors.background = Some(color);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn print_image(&mut self, image: &Image, options: &PrintOptions) -> Result<(), PrintImageError> {
        let key = (options.cursor_position.row, options.cursor_position.column);
        let image = PrintedImage { image: image.clone(), width_columns: options.columns };
        self.images.insert(key, image);
        Ok(())
    }
}

impl TerminalIo for VirtualTerminal {
    fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
        use TerminalCommand::*;
        match command {
            BeginUpdate | EndUpdate => (),
            MoveTo { column, row } => self.move_to(*column, *row)?,
            MoveToRow(row) => self.move_to_row(*row)?,
            MoveToColumn(column) => self.move_to_column(*column)?,
            MoveDown(amount) => self.move_down(*amount)?,
            MoveRight(amount) => self.move_right(*amount)?,
            MoveToNextLine => self.move_to_next_line()?,
            PrintText { content, style } => self.print_text(content, style)?,
            ClearScreen => self.clear_screen()?,
            SetColors(colors) => self.set_colors(*colors)?,
            SetBackgroundColor(color) => self.set_background_color(*color)?,
            Flush => self.flush()?,
            PrintImage { image, options } => self.print_image(image, options)?,
        };
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.row
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StyledChar {
    pub(crate) character: char,
    pub(crate) style: TextStyle,
}

impl Default for StyledChar {
    fn default() -> Self {
        Self { character: ' ', style: Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait TerminalGridExt {
        fn assert_contents(&self, lines: &[&str]);
    }

    impl TerminalGridExt for TerminalGrid {
        fn assert_contents(&self, lines: &[&str]) {
            assert_eq!(self.rows.len(), lines.len());
            for (line, expected) in self.rows.iter().zip(lines) {
                let line: String = line.iter().map(|c| c.character).collect();
                assert_eq!(line, *expected);
            }
        }
    }

    #[test]
    fn text() {
        let dimensions = WindowSize { rows: 2, columns: 3, height: 0, width: 0 };
        let mut term = VirtualTerminal::new(dimensions);
        for c in "abc".chars() {
            term.print_text(&c.to_string(), &Default::default()).expect("print failed");
        }
        term.move_to_next_line().unwrap();
        term.print_text("A", &Default::default()).expect("print failed");
        let grid = term.into_contents();
        grid.assert_contents(&["abc", "A  "]);
    }

    #[test]
    fn movement() {
        let dimensions = WindowSize { rows: 2, columns: 3, height: 0, width: 0 };
        let mut term = VirtualTerminal::new(dimensions);
        term.print_text("A", &Default::default()).unwrap();
        term.move_down(1).unwrap();
        term.print_text("B", &Default::default()).unwrap();
        term.move_to(2, 0).unwrap();
        term.print_text("C", &Default::default()).unwrap();
        term.move_to_row(1).unwrap();
        term.move_to_column(2).unwrap();
        term.print_text("D", &Default::default()).unwrap();

        let grid = term.into_contents();
        grid.assert_contents(&["A C", " BD"]);
    }

    #[test]
    fn iterator() {
        let row = &[
            StyledChar { character: ' ', style: TextStyle::default() },
            StyledChar { character: 'A', style: TextStyle::default() },
            StyledChar { character: 'B', style: TextStyle::default().bold() },
            StyledChar { character: 'C', style: TextStyle::default().bold() },
            StyledChar { character: 'D', style: TextStyle::default() },
        ];
        let texts: Vec<_> = TerminalRowIterator::new(row).collect();
        assert_eq!(texts, &[Text::from(" A"), Text::new("BC", TextStyle::default().bold()), Text::from("D")]);
    }
}
