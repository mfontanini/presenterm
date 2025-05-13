use super::{
    RenderError, RenderResult, layout::Layout, operation::ImagePosition, properties::CursorPosition, text::TextDrawer,
};
use crate::{
    config::{MaxColumnsAlignment, MaxRowsAlignment},
    markdown::{text::WeightedLine, text_style::Colors},
    render::{
        layout::Positioning,
        operation::{
            AsRenderOperations, BlockLine, ImageRenderProperties, ImageSize, MarginProperties, RenderAsync,
            RenderOperation,
        },
        properties::WindowSize,
    },
    terminal::{
        image::{
            Image,
            printer::{ImageProperties, PrintOptions},
            scale::{ImageScaler, ScaleImage},
        },
        printer::{TerminalCommand, TerminalIo},
    },
    theme::Alignment,
};
use std::mem;

const MINIMUM_LINE_LENGTH: u16 = 10;

#[derive(Clone, Debug)]
pub(crate) struct MaxSize {
    pub(crate) max_columns: u16,
    pub(crate) max_columns_alignment: MaxColumnsAlignment,
    pub(crate) max_rows: u16,
    pub(crate) max_rows_alignment: MaxRowsAlignment,
}

impl Default for MaxSize {
    fn default() -> Self {
        Self {
            max_columns: u16::MAX,
            max_columns_alignment: Default::default(),
            max_rows: u16::MAX,
            max_rows_alignment: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RenderEngineOptions {
    pub(crate) validate_overflows: bool,
    pub(crate) max_size: MaxSize,
    pub(crate) column_layout_margin: u16,
}

impl Default for RenderEngineOptions {
    fn default() -> Self {
        Self { validate_overflows: false, max_size: Default::default(), column_layout_margin: 4 }
    }
}

pub(crate) struct RenderEngine<'a, T>
where
    T: TerminalIo,
{
    terminal: &'a mut T,
    window_rects: Vec<WindowRect>,
    colors: Colors,
    max_modified_row: u16,
    layout: LayoutState,
    options: RenderEngineOptions,
    image_scaler: Box<dyn ScaleImage>,
}

impl<'a, T> RenderEngine<'a, T>
where
    T: TerminalIo,
{
    pub(crate) fn new(terminal: &'a mut T, window_dimensions: WindowSize, options: RenderEngineOptions) -> Self {
        let max_modified_row = terminal.cursor_row();
        let current_rect = Self::starting_rect(window_dimensions, &options);
        let window_rects = vec![current_rect.clone()];
        Self {
            terminal,
            window_rects,
            colors: Default::default(),
            max_modified_row,
            layout: Default::default(),
            options,
            image_scaler: Box::<ImageScaler>::default(),
        }
    }

    fn starting_rect(mut dimensions: WindowSize, options: &RenderEngineOptions) -> WindowRect {
        let mut start_row = 0;
        let mut start_column = 0;
        if dimensions.columns > options.max_size.max_columns {
            let extra_width = dimensions.columns - options.max_size.max_columns;
            dimensions = dimensions.shrink_columns(extra_width);
            start_column = match options.max_size.max_columns_alignment {
                MaxColumnsAlignment::Left => 0,
                MaxColumnsAlignment::Center => extra_width / 2,
                MaxColumnsAlignment::Right => extra_width,
            };
        }
        if dimensions.rows > options.max_size.max_rows {
            let extra_height = dimensions.rows - options.max_size.max_rows;
            dimensions = dimensions.shrink_rows(extra_height);
            start_row = match options.max_size.max_rows_alignment {
                MaxRowsAlignment::Top => 0,
                MaxRowsAlignment::Center => extra_height / 2,
                MaxRowsAlignment::Bottom => extra_height,
            };
        }
        WindowRect { dimensions, start_column, start_row }
    }

    pub(crate) fn render<'b>(mut self, operations: impl Iterator<Item = &'b RenderOperation>) -> RenderResult {
        let current_rect = self.current_rect().clone();
        self.terminal.execute(&TerminalCommand::SetCursorBoundaries { rows: current_rect.dimensions.rows })?;
        self.terminal.execute(&TerminalCommand::BeginUpdate)?;
        if current_rect.start_row != 0 || current_rect.start_column != 0 {
            self.terminal
                .execute(&TerminalCommand::MoveTo { column: current_rect.start_column, row: current_rect.start_row })?;
        }
        for operation in operations {
            self.render_one(operation)?;
        }
        self.terminal.execute(&TerminalCommand::EndUpdate)?;
        self.terminal.execute(&TerminalCommand::Flush)?;
        if self.options.validate_overflows && self.max_modified_row > self.window_rects[0].dimensions.rows {
            return Err(RenderError::VerticalOverflow);
        }
        Ok(())
    }

    fn render_one(&mut self, operation: &RenderOperation) -> RenderResult {
        match operation {
            RenderOperation::ClearScreen => self.clear_screen(),
            RenderOperation::ApplyMargin(properties) => self.apply_margin(properties),
            RenderOperation::PopMargin => self.pop_margin(),
            RenderOperation::SetColors(colors) => self.set_colors(colors),
            RenderOperation::JumpToVerticalCenter => self.jump_to_vertical_center(),
            RenderOperation::JumpToRow { index } => self.jump_to_row(*index),
            RenderOperation::JumpToBottomRow { index } => self.jump_to_bottom(*index),
            RenderOperation::JumpToColumn { index } => self.jump_to_column(*index),
            RenderOperation::RenderText { line, alignment } => self.render_text(line, *alignment),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image, properties) => self.render_image(image, properties),
            RenderOperation::RenderBlockLine(operation) => self.render_block_line(operation),
            RenderOperation::RenderDynamic(generator) => self.render_dynamic(generator.as_ref()),
            RenderOperation::RenderAsync(generator) => self.render_async(generator.as_ref()),
            RenderOperation::InitColumnLayout { columns } => self.init_column_layout(columns),
            RenderOperation::EnterColumn { column } => self.enter_column(*column),
            RenderOperation::ExitLayout => self.exit_layout(),
        }?;
        if let LayoutState::EnteredColumn { column, columns } = &mut self.layout {
            columns[*column].current_row = self.terminal.cursor_row();
        };
        self.max_modified_row = self.max_modified_row.max(self.terminal.cursor_row());
        Ok(())
    }

    fn current_rect(&self) -> &WindowRect {
        // This invariant is enforced when popping.
        self.window_rects.last().expect("no rects")
    }

    fn current_dimensions(&self) -> &WindowSize {
        &self.current_rect().dimensions
    }

    fn clear_screen(&mut self) -> RenderResult {
        let current = self.current_rect().clone();
        self.terminal.execute(&TerminalCommand::ClearScreen)?;
        self.terminal.execute(&TerminalCommand::MoveTo { column: current.start_column, row: current.start_row })?;
        self.max_modified_row = 0;
        Ok(())
    }

    fn apply_margin(&mut self, properties: &MarginProperties) -> RenderResult {
        let MarginProperties { horizontal: horizontal_margin, top, bottom } = properties;
        let current = self.current_rect();
        let margin = horizontal_margin.as_characters(current.dimensions.columns);
        let new_rect = current.shrink_horizontal(margin).shrink_bottom(*bottom).shrink_top(*top);
        if new_rect.start_row != self.terminal.cursor_row() {
            self.terminal.execute(&TerminalCommand::MoveToRow(new_rect.start_row))?;
        }
        self.window_rects.push(new_rect);
        Ok(())
    }

    fn pop_margin(&mut self) -> RenderResult {
        if self.window_rects.len() == 1 {
            return Err(RenderError::PopDefaultScreen);
        }
        self.window_rects.pop();
        Ok(())
    }

    fn set_colors(&mut self, colors: &Colors) -> RenderResult {
        self.colors = *colors;
        self.apply_colors()
    }

    fn apply_colors(&mut self) -> RenderResult {
        self.terminal.execute(&TerminalCommand::SetColors(self.colors))?;
        Ok(())
    }

    fn jump_to_vertical_center(&mut self) -> RenderResult {
        let current = self.current_rect();
        let center_row = current.dimensions.rows / 2;
        let center_row = center_row.saturating_add(current.start_row);
        self.terminal.execute(&TerminalCommand::MoveToRow(center_row))?;
        Ok(())
    }

    fn jump_to_row(&mut self, row: u16) -> RenderResult {
        // Make this relative to the beginning of the current rect.
        let row = self.current_rect().start_row.saturating_add(row);
        self.terminal.execute(&TerminalCommand::MoveToRow(row))?;
        Ok(())
    }

    fn jump_to_bottom(&mut self, index: u16) -> RenderResult {
        let current = self.current_rect();
        let target_row = current.dimensions.rows.saturating_sub(index).saturating_sub(1);
        let target_row = target_row.saturating_add(current.start_row);
        self.terminal.execute(&TerminalCommand::MoveToRow(target_row))?;
        Ok(())
    }

    fn jump_to_column(&mut self, column: u16) -> RenderResult {
        // Make this relative to the beginning of the current rect.
        let column = self.current_rect().start_column.saturating_add(column);
        self.terminal.execute(&TerminalCommand::MoveToColumn(column))?;
        Ok(())
    }

    fn render_text(&mut self, text: &WeightedLine, alignment: Alignment) -> RenderResult {
        let layout = self.build_layout(alignment);
        let dimensions = self.current_dimensions();
        let positioning = layout.compute(dimensions, text.width() as u16);
        let prefix = "".into();
        let text_drawer = TextDrawer::new(&prefix, 0, text, positioning, &self.colors, MINIMUM_LINE_LENGTH)?;
        let center_newlines = matches!(alignment, Alignment::Center { .. });
        let text_drawer = text_drawer.center_newlines(center_newlines);
        text_drawer.draw(self.terminal)?;
        // Restore colors
        self.apply_colors()
    }

    fn render_line_break(&mut self) -> RenderResult {
        self.terminal.execute(&TerminalCommand::MoveToNextLine)?;
        Ok(())
    }

    fn render_image(&mut self, image: &Image, properties: &ImageRenderProperties) -> RenderResult {
        let rect = self.current_rect().clone();
        let starting_row = self.terminal.cursor_row();
        let starting_cursor =
            CursorPosition { row: starting_row.saturating_sub(rect.start_row), column: rect.start_column };

        let (width, height) = image.image().dimensions();
        let (columns, rows) = match properties.size {
            ImageSize::ShrinkIfNeeded => {
                let image_scale =
                    self.image_scaler.fit_image_to_rect(&rect.dimensions, width, height, &starting_cursor);
                (image_scale.columns, image_scale.rows)
            }
            ImageSize::Specific(columns, rows) => (columns, rows),
            ImageSize::WidthScaled { ratio } => {
                let extra_columns = (rect.dimensions.columns as f64 * (1.0 - ratio)).ceil() as u16;
                let dimensions = rect.dimensions.shrink_columns(extra_columns);
                let image_scale =
                    self.image_scaler.scale_image(&dimensions, &rect.dimensions, width, height, &starting_cursor);
                (image_scale.columns, image_scale.rows)
            }
        };
        let cursor = match &properties.position {
            ImagePosition::Cursor => starting_cursor.clone(),
            ImagePosition::Center => Self::center_cursor(columns, &rect.dimensions, &starting_cursor),
            ImagePosition::Right => Self::align_cursor_right(columns, &rect.dimensions, &starting_cursor),
        };
        self.terminal.execute(&TerminalCommand::MoveToColumn(cursor.column))?;

        let options = PrintOptions {
            columns,
            rows,
            z_index: properties.z_index,
            column_width: rect.dimensions.pixels_per_column() as u16,
            row_height: rect.dimensions.pixels_per_row() as u16,
            background_color: properties.background_color,
        };
        self.terminal.execute(&TerminalCommand::PrintImage { image: image.clone(), options })?;
        if properties.restore_cursor {
            self.terminal.execute(&TerminalCommand::MoveTo { column: starting_cursor.column, row: starting_row })?;
        } else {
            self.terminal.execute(&TerminalCommand::MoveToRow(starting_row + rows))?;
        }
        self.apply_colors()
    }

    fn center_cursor(columns: u16, window: &WindowSize, cursor: &CursorPosition) -> CursorPosition {
        let start_column = window.columns / 2 - (columns / 2);
        let start_column = start_column + cursor.column;
        CursorPosition { row: cursor.row, column: start_column }
    }

    fn align_cursor_right(columns: u16, window: &WindowSize, cursor: &CursorPosition) -> CursorPosition {
        let start_column = window.columns.saturating_sub(columns).saturating_add(cursor.column);
        CursorPosition { row: cursor.row, column: start_column }
    }

    fn render_block_line(&mut self, operation: &BlockLine) -> RenderResult {
        let BlockLine {
            text,
            block_length,
            alignment,
            block_color,
            prefix,
            right_padding_length,
            repeat_prefix_on_wrap,
        } = operation;
        let layout = self.build_layout(*alignment).with_font_size(text.font_size());

        let dimensions = self.current_dimensions();
        let Positioning { max_line_length, start_column } = layout.compute(dimensions, *block_length);
        if self.options.validate_overflows && text.width() as u16 > max_line_length {
            return Err(RenderError::HorizontalOverflow);
        }

        self.terminal.execute(&TerminalCommand::MoveToColumn(start_column))?;

        let positioning = Positioning { max_line_length, start_column };
        let text_drawer =
            TextDrawer::new(prefix, *right_padding_length, text, positioning, &self.colors, MINIMUM_LINE_LENGTH)?
                .with_surrounding_block(*block_color)
                .repeat_prefix_on_wrap(*repeat_prefix_on_wrap);
        text_drawer.draw(self.terminal)?;

        // Restore colors
        self.apply_colors()?;
        Ok(())
    }

    fn render_dynamic(&mut self, generator: &dyn AsRenderOperations) -> RenderResult {
        let operations = generator.as_render_operations(self.current_dimensions());
        for operation in operations {
            self.render_one(&operation)?;
        }
        Ok(())
    }

    fn render_async(&mut self, generator: &dyn RenderAsync) -> RenderResult {
        let operations = generator.as_render_operations(self.current_dimensions());
        for operation in operations {
            self.render_one(&operation)?;
        }
        Ok(())
    }

    fn init_column_layout(&mut self, columns: &[u8]) -> RenderResult {
        if !matches!(self.layout, LayoutState::Default) {
            self.exit_layout()?;
        }
        let columns = columns
            .iter()
            .map(|width| Column { width: *width as u16, current_row: self.terminal.cursor_row() })
            .collect();
        self.layout = LayoutState::InitializedColumn { columns };
        Ok(())
    }

    fn enter_column(&mut self, column_index: usize) -> RenderResult {
        let columns = match mem::take(&mut self.layout) {
            LayoutState::Default => return Err(RenderError::InvalidLayoutEnter),
            LayoutState::InitializedColumn { columns, .. } | LayoutState::EnteredColumn { columns, .. }
                if column_index >= columns.len() =>
            {
                return Err(RenderError::InvalidLayoutEnter);
            }
            LayoutState::InitializedColumn { columns } => columns,
            LayoutState::EnteredColumn { columns, .. } => {
                // Pop this one and start clean
                self.pop_margin()?;
                columns
            }
        };
        let total_column_units: u16 = columns.iter().map(|c| c.width).sum();
        let column_units_before: u16 = columns.iter().take(column_index).map(|c| c.width).sum();
        let current_rect = self.current_rect();
        let unit_width = current_rect.dimensions.columns as f64 / total_column_units as f64;
        let start_column = current_rect.start_column + (unit_width * column_units_before as f64) as u16;
        let start_row = columns[column_index].current_row;
        let new_column_count = (total_column_units - columns[column_index].width) * unit_width as u16;
        let new_size = current_rect
            .dimensions
            .shrink_columns(new_column_count)
            .shrink_rows(start_row.saturating_sub(current_rect.start_row));
        let mut dimensions = WindowRect { dimensions: new_size, start_column, start_row };
        // Shrink every column's right edge except for last
        if column_index < columns.len() - 1 {
            dimensions = dimensions.shrink_right(self.options.column_layout_margin);
        }
        // Shrink every column's left edge except for first
        if column_index > 0 {
            dimensions = dimensions.shrink_left(self.options.column_layout_margin);
        }

        self.window_rects.push(dimensions);
        self.terminal.execute(&TerminalCommand::MoveToRow(start_row))?;
        self.layout = LayoutState::EnteredColumn { column: column_index, columns };
        Ok(())
    }

    fn exit_layout(&mut self) -> RenderResult {
        match &self.layout {
            LayoutState::Default | LayoutState::InitializedColumn { .. } => Ok(()),
            LayoutState::EnteredColumn { .. } => {
                self.terminal.execute(&TerminalCommand::MoveTo { column: 0, row: self.max_modified_row })?;
                self.layout = LayoutState::Default;
                self.pop_margin()?;
                Ok(())
            }
        }
    }

    fn build_layout(&self, alignment: Alignment) -> Layout {
        Layout::new(alignment).with_start_column(self.current_rect().start_column)
    }
}

#[derive(Default)]
enum LayoutState {
    #[default]
    Default,
    InitializedColumn {
        columns: Vec<Column>,
    },
    EnteredColumn {
        column: usize,
        columns: Vec<Column>,
    },
}

struct Column {
    width: u16,
    current_row: u16,
}

#[derive(Clone, Debug)]
struct WindowRect {
    dimensions: WindowSize,
    start_column: u16,
    start_row: u16,
}

impl WindowRect {
    fn shrink_horizontal(&self, margin: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(margin.saturating_mul(2));
        let start_column = self.start_column + margin;
        Self { dimensions, start_column, start_row: self.start_row }
    }

    fn shrink_left(&self, size: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(size);
        let start_column = self.start_column.saturating_add(size);
        Self { dimensions, start_column, start_row: self.start_row }
    }

    fn shrink_right(&self, size: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(size);
        Self { dimensions, start_column: self.start_column, start_row: self.start_row }
    }

    fn shrink_top(&self, rows: u16) -> Self {
        let dimensions = self.dimensions.shrink_rows(rows);
        let start_row = self.start_row.saturating_add(rows);
        Self { dimensions, start_column: self.start_column, start_row }
    }

    fn shrink_bottom(&self, rows: u16) -> Self {
        let dimensions = self.dimensions.shrink_rows(rows);
        Self { dimensions, start_column: self.start_column, start_row: self.start_row }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        markdown::text_style::{Color, TextStyle},
        terminal::{
            image::{
                ImageSource,
                printer::{PrintImageError, TerminalImage},
                scale::TerminalRect,
            },
            printer::TerminalError,
        },
        theme::Margin,
    };
    use ::image::{ColorType, DynamicImage};
    use rstest::rstest;
    use std::io;
    use unicode_width::UnicodeWidthStr;

    #[derive(Debug, PartialEq)]
    enum Instruction {
        MoveTo(u16, u16),
        MoveToRow(u16),
        MoveToColumn(u16),
        MoveDown(u16),
        MoveRight(u16),
        MoveLeft(u16),
        MoveToNextLine,
        PrintText(String),
        ClearScreen,
        SetBackgroundColor(Color),
        PrintImage(PrintOptions),
    }

    #[derive(Default)]
    struct TerminalBuf {
        instructions: Vec<Instruction>,
        cursor_row: u16,
    }

    impl TerminalBuf {
        fn push(&mut self, instruction: Instruction) -> io::Result<()> {
            self.instructions.push(instruction);
            Ok(())
        }

        fn move_to(&mut self, column: u16, row: u16) -> io::Result<()> {
            self.cursor_row = row;
            self.push(Instruction::MoveTo(column, row))
        }

        fn move_to_row(&mut self, row: u16) -> io::Result<()> {
            self.cursor_row = row;
            self.push(Instruction::MoveToRow(row))
        }

        fn move_to_column(&mut self, column: u16) -> io::Result<()> {
            self.push(Instruction::MoveToColumn(column))
        }

        fn move_down(&mut self, amount: u16) -> io::Result<()> {
            self.push(Instruction::MoveDown(amount))
        }

        fn move_right(&mut self, amount: u16) -> io::Result<()> {
            self.push(Instruction::MoveRight(amount))
        }

        fn move_left(&mut self, amount: u16) -> io::Result<()> {
            self.push(Instruction::MoveLeft(amount))
        }

        fn move_to_next_line(&mut self) -> io::Result<()> {
            self.push(Instruction::MoveToNextLine)
        }

        fn print_text(&mut self, content: &str, _style: &TextStyle) -> io::Result<()> {
            let content = content.to_string();
            if content.is_empty() {
                return Ok(());
            }
            self.cursor_row = content.width() as u16;
            self.push(Instruction::PrintText(content))
        }

        fn clear_screen(&mut self) -> io::Result<()> {
            self.cursor_row = 0;
            self.push(Instruction::ClearScreen)
        }

        fn set_colors(&mut self, _colors: Colors) -> io::Result<()> {
            Ok(())
        }

        fn set_background_color(&mut self, color: Color) -> io::Result<()> {
            self.push(Instruction::SetBackgroundColor(color))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn print_image(&mut self, _image: &Image, options: &PrintOptions) -> Result<(), PrintImageError> {
            let _ = self.push(Instruction::PrintImage(options.clone()));
            Ok(())
        }
    }

    impl TerminalIo for TerminalBuf {
        fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
            use TerminalCommand::*;
            match command {
                BeginUpdate => (),
                EndUpdate => (),
                MoveTo { column, row } => self.move_to(*column, *row)?,
                MoveToRow(row) => self.move_to_row(*row)?,
                MoveToColumn(column) => self.move_to_column(*column)?,
                MoveDown(amount) => self.move_down(*amount)?,
                MoveRight(amount) => self.move_right(*amount)?,
                MoveLeft(amount) => self.move_left(*amount)?,
                MoveToNextLine => self.move_to_next_line()?,
                PrintText { content, style } => self.print_text(content, style)?,
                ClearScreen => self.clear_screen()?,
                SetColors(colors) => self.set_colors(*colors)?,
                SetBackgroundColor(color) => self.set_background_color(*color)?,
                Flush => self.flush()?,
                PrintImage { image, options } => self.print_image(image, options)?,
                SetCursorBoundaries { .. } => (),
            };
            Ok(())
        }

        fn cursor_row(&self) -> u16 {
            self.cursor_row
        }
    }

    struct DummyImageScaler;

    impl ScaleImage for DummyImageScaler {
        fn scale_image(
            &self,
            _scale_size: &WindowSize,
            _window_dimensions: &WindowSize,
            image_width: u32,
            image_height: u32,
            _position: &CursorPosition,
        ) -> TerminalRect {
            TerminalRect { rows: image_width as u16, columns: image_height as u16 }
        }

        fn fit_image_to_rect(
            &self,
            _dimensions: &WindowSize,
            image_width: u32,
            image_height: u32,
            _position: &CursorPosition,
        ) -> TerminalRect {
            TerminalRect { rows: image_width as u16, columns: image_height as u16 }
        }
    }

    fn do_render(max_size: MaxSize, operations: &[RenderOperation]) -> Vec<Instruction> {
        let mut buf = TerminalBuf::default();
        let dimensions = WindowSize { rows: 100, columns: 100, height: 200, width: 200 };
        let options = RenderEngineOptions { validate_overflows: false, max_size, column_layout_margin: 0 };
        let mut engine = RenderEngine::new(&mut buf, dimensions, options);
        engine.image_scaler = Box::new(DummyImageScaler);
        engine.render(operations.iter()).expect("render failed");
        buf.instructions
    }

    fn render(operations: &[RenderOperation]) -> Vec<Instruction> {
        do_render(Default::default(), operations)
    }

    fn render_with_max_size(operations: &[RenderOperation]) -> Vec<Instruction> {
        let max_size = MaxSize {
            max_rows: 10,
            max_rows_alignment: MaxRowsAlignment::Center,
            max_columns: 20,
            max_columns_alignment: MaxColumnsAlignment::Center,
        };
        do_render(max_size, operations)
    }

    #[test]
    fn columns() {
        let ops = render(&[
            RenderOperation::InitColumnLayout { columns: vec![1, 1] },
            // print on column 0
            RenderOperation::EnterColumn { column: 0 },
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            // print on column 1
            RenderOperation::EnterColumn { column: 1 },
            RenderOperation::RenderText { line: "B".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            // go back to column 0 and print
            RenderOperation::EnterColumn { column: 0 },
            RenderOperation::RenderText { line: "1".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [
            Instruction::MoveToRow(0),
            Instruction::MoveToColumn(0),
            Instruction::PrintText("A".into()),
            Instruction::MoveToRow(0),
            Instruction::MoveToColumn(50),
            Instruction::PrintText("B".into()),
            // when we go back we should proceed from where we left off (row == 1)
            Instruction::MoveToRow(1),
            Instruction::MoveToColumn(0),
            Instruction::PrintText("1".into()),
        ];
        assert_eq!(ops, expected);
    }

    #[test]
    fn bottom_margin() {
        let ops = render(&[
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 0, bottom: 10 }),
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            RenderOperation::JumpToBottomRow { index: 0 },
            RenderOperation::RenderText { line: "B".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [
            Instruction::MoveToColumn(1),
            Instruction::PrintText("A".into()),
            // 100 - 10 (bottom margin)
            Instruction::MoveToRow(89),
            Instruction::MoveToColumn(1),
            Instruction::PrintText("B".into()),
        ];
        assert_eq!(ops, expected);
    }

    #[test]
    fn top_margin() {
        let ops = render(&[
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 3, bottom: 0 }),
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [Instruction::MoveToRow(3), Instruction::MoveToColumn(1), Instruction::PrintText("A".into())];
        assert_eq!(ops, expected);
    }

    #[test]
    fn margins() {
        let ops = render(&[
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 3, bottom: 10 }),
            RenderOperation::JumpToRow { index: 0 },
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            RenderOperation::JumpToBottomRow { index: 0 },
            RenderOperation::RenderText { line: "B".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [
            Instruction::MoveToRow(3),
            Instruction::MoveToRow(3),
            Instruction::MoveToColumn(1),
            Instruction::PrintText("A".into()),
            // 100 - 10 (bottom margin)
            Instruction::MoveToRow(89),
            Instruction::MoveToColumn(1),
            Instruction::PrintText("B".into()),
        ];
        assert_eq!(ops, expected);
    }

    #[test]
    fn nested_margins() {
        let ops = render(&[
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 0, bottom: 10 }),
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 0, bottom: 10 }),
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            RenderOperation::JumpToBottomRow { index: 0 },
            RenderOperation::RenderText { line: "B".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            // pop and go to bottom, this should go back up to the end of the first margin
            RenderOperation::PopMargin,
            RenderOperation::JumpToBottomRow { index: 0 },
            RenderOperation::RenderText { line: "C".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [
            Instruction::MoveToColumn(2),
            Instruction::PrintText("A".into()),
            // 100 - 10 (margin) - 10 (second margin)
            Instruction::MoveToRow(79),
            Instruction::MoveToColumn(2),
            Instruction::PrintText("B".into()),
            // 100 - 10 (margin)
            Instruction::MoveToRow(89),
            Instruction::MoveToColumn(1),
            Instruction::PrintText("C".into()),
        ];
        assert_eq!(ops, expected);
    }

    #[test]
    fn margin_with_max_size() {
        let ops = render_with_max_size(&[
            RenderOperation::RenderText { line: "A".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            RenderOperation::ApplyMargin(MarginProperties { horizontal: Margin::Fixed(1), top: 2, bottom: 1 }),
            RenderOperation::RenderText { line: "B".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
            RenderOperation::JumpToBottomRow { index: 0 },
            RenderOperation::RenderText { line: "C".into(), alignment: Alignment::Left { margin: Margin::Fixed(0) } },
        ]);
        let expected = [
            // centered 20x10
            Instruction::MoveTo(40, 45),
            Instruction::MoveToColumn(40),
            Instruction::PrintText("A".into()),
            // jump 2 down because of top margin
            Instruction::MoveToRow(47),
            // jump 1 right because of horizontal margin
            Instruction::MoveToColumn(41),
            Instruction::PrintText("B".into()),
            // rows go from 47 to 53 (7 total)
            Instruction::MoveToRow(53),
            Instruction::MoveToColumn(41),
            Instruction::PrintText("C".into()),
        ];
        assert_eq!(ops, expected);
    }

    // print the same 2x2 image with all size configs, they should all yield the same
    #[rstest]
    #[case::shrink(ImageSize::ShrinkIfNeeded)]
    #[case::specific(ImageSize::Specific(2, 2))]
    #[case::width_scaled(ImageSize::WidthScaled { ratio: 1.0 })]
    fn image(#[case] size: ImageSize) {
        let image = DynamicImage::new(2, 2, ColorType::Rgba8);
        let image = Image::new(TerminalImage::Ascii(image.into()), ImageSource::Generated);
        let properties = ImageRenderProperties {
            z_index: 0,
            size,
            restore_cursor: false,
            background_color: None,
            position: ImagePosition::Cursor,
        };
        let ops = render_with_max_size(&[RenderOperation::RenderImage(image, properties)]);
        let expected = [
            // centered 20x10, the image is 2x2 so we stand one away from center
            Instruction::MoveTo(40, 45),
            Instruction::MoveToColumn(40),
            Instruction::PrintImage(PrintOptions {
                columns: 2,
                rows: 2,
                z_index: 0,
                background_color: None,
                column_width: 2,
                row_height: 2,
            }),
            // place cursor after the image
            Instruction::MoveToRow(47),
        ];
        assert_eq!(ops, expected);
    }

    // same as the above but center it
    #[rstest]
    #[case::shrink(ImageSize::ShrinkIfNeeded)]
    #[case::specific(ImageSize::Specific(2, 2))]
    #[case::width_scaled(ImageSize::WidthScaled { ratio: 1.0 })]
    fn centered_image(#[case] size: ImageSize) {
        let image = DynamicImage::new(2, 2, ColorType::Rgba8);
        let image = Image::new(TerminalImage::Ascii(image.into()), ImageSource::Generated);
        let properties = ImageRenderProperties {
            z_index: 0,
            size,
            restore_cursor: false,
            background_color: None,
            position: ImagePosition::Center,
        };
        let ops = render_with_max_size(&[RenderOperation::RenderImage(image, properties)]);
        let expected = [
            // centered 20x10, the image is 2x2 so we stand one away from center
            Instruction::MoveTo(40, 45),
            Instruction::MoveToColumn(49),
            Instruction::PrintImage(PrintOptions {
                columns: 2,
                rows: 2,
                z_index: 0,
                background_color: None,
                column_width: 2,
                row_height: 2,
            }),
            // place cursor after the image
            Instruction::MoveToRow(47),
        ];
        assert_eq!(ops, expected);
    }

    // same as the above but use right alignment
    #[rstest]
    #[case::shrink(ImageSize::ShrinkIfNeeded)]
    #[case::specific(ImageSize::Specific(2, 2))]
    #[case::width_scaled(ImageSize::WidthScaled { ratio: 1.0 })]
    fn right_aligned_image(#[case] size: ImageSize) {
        let image = DynamicImage::new(2, 2, ColorType::Rgba8);
        let image = Image::new(TerminalImage::Ascii(image.into()), ImageSource::Generated);
        let properties = ImageRenderProperties {
            z_index: 0,
            size,
            restore_cursor: false,
            background_color: None,
            position: ImagePosition::Right,
        };
        let ops = render_with_max_size(&[RenderOperation::RenderImage(image, properties)]);
        let expected = [
            // right aligned 20x10, the image is 2x2 so we stand one away from the right
            Instruction::MoveTo(40, 45),
            Instruction::MoveToColumn(58),
            Instruction::PrintImage(PrintOptions {
                columns: 2,
                rows: 2,
                z_index: 0,
                background_color: None,
                column_width: 2,
                row_height: 2,
            }),
            // place cursor after the image
            Instruction::MoveToRow(47),
        ];
        assert_eq!(ops, expected);
    }

    // same as the above but center it
    #[rstest]
    fn restore_cursor_after_image() {
        let image = DynamicImage::new(2, 2, ColorType::Rgba8);
        let image = Image::new(TerminalImage::Ascii(image.into()), ImageSource::Generated);
        let properties = ImageRenderProperties {
            z_index: 0,
            size: ImageSize::ShrinkIfNeeded,
            restore_cursor: true,
            background_color: None,
            position: ImagePosition::Center,
        };
        let ops = render_with_max_size(&[RenderOperation::RenderImage(image, properties)]);
        let expected = [
            // centered 20x10, the image is 2x2 so we stand one away from center
            Instruction::MoveTo(40, 45),
            Instruction::MoveToColumn(49),
            Instruction::PrintImage(PrintOptions {
                columns: 2,
                rows: 2,
                z_index: 0,
                background_color: None,
                column_width: 2,
                row_height: 2,
            }),
            // place cursor after the image
            Instruction::MoveTo(40, 45),
        ];
        assert_eq!(ops, expected);
    }
}
