use super::{
    draw::{RenderError, RenderResult},
    layout::Layout,
    properties::CursorPosition,
    terminal::{Terminal, TerminalWrite},
    text::TextDrawer,
};
use crate::{
    markdown::text::WeightedTextBlock,
    media::{
        image::Image,
        printer::{PrintOptions, ResourceProperties},
        scale::scale_image,
    },
    presentation::{
        AsRenderOperations, ImageProperties, ImageSize, MarginProperties, PreformattedLine, RenderAsync,
        RenderOperation,
    },
    render::{layout::Positioning, properties::WindowSize},
    style::Colors,
    theme::Alignment,
};
use std::mem;

#[derive(Debug, Default)]
pub(crate) struct RenderEngineOptions {
    pub(crate) validate_overflows: bool,
}

pub(crate) struct RenderEngine<'a, W>
where
    W: TerminalWrite,
{
    terminal: &'a mut Terminal<W>,
    window_rects: Vec<WindowRect>,
    colors: Colors,
    max_modified_row: u16,
    layout: LayoutState,
    options: RenderEngineOptions,
}

impl<'a, W> RenderEngine<'a, W>
where
    W: TerminalWrite,
{
    pub(crate) fn new(
        terminal: &'a mut Terminal<W>,
        window_dimensions: WindowSize,
        options: RenderEngineOptions,
    ) -> Self {
        let max_modified_row = terminal.cursor_row;
        let current_rect = WindowRect { dimensions: window_dimensions, start_column: 0 };
        let window_rects = vec![current_rect.clone()];
        Self {
            terminal,
            window_rects,
            colors: Default::default(),
            max_modified_row,
            layout: Default::default(),
            options,
        }
    }

    pub(crate) fn render<'b>(mut self, operations: impl Iterator<Item = &'b RenderOperation>) -> RenderResult {
        self.terminal.begin_update()?;
        for operation in operations {
            self.render_one(operation)?;
        }
        self.terminal.end_update()?;
        self.terminal.flush()?;
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
            RenderOperation::RenderText { line, alignment } => self.render_text(line, alignment),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image, properties) => self.render_image(image, properties),
            RenderOperation::RenderPreformattedLine(operation) => self.render_preformatted_line(operation),
            RenderOperation::RenderDynamic(generator) => self.render_dynamic(generator.as_ref()),
            RenderOperation::RenderAsync(generator) => self.render_async(generator.as_ref()),
            RenderOperation::InitColumnLayout { columns } => self.init_column_layout(columns),
            RenderOperation::EnterColumn { column } => self.enter_column(*column),
            RenderOperation::ExitLayout => self.exit_layout(),
        }?;
        self.max_modified_row = self.max_modified_row.max(self.terminal.cursor_row);
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
        self.terminal.clear_screen()?;
        self.terminal.move_to(0, 0)?;
        self.max_modified_row = 0;
        Ok(())
    }

    fn apply_margin(&mut self, properties: &MarginProperties) -> RenderResult {
        let MarginProperties { horizontal_margin, bottom_slide_margin } = properties;
        let current = self.current_rect();
        let margin = horizontal_margin.as_characters(current.dimensions.columns);
        let new_rect = current.apply_margin(margin).shrink_rows(*bottom_slide_margin);
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
        self.colors = colors.clone();
        self.apply_colors()
    }

    fn apply_colors(&mut self) -> RenderResult {
        self.terminal.set_colors(self.colors.clone())?;
        Ok(())
    }

    fn jump_to_vertical_center(&mut self) -> RenderResult {
        let center_row = self.current_dimensions().rows / 2;
        self.terminal.move_to_row(center_row)?;
        Ok(())
    }

    fn jump_to_row(&mut self, index: u16) -> RenderResult {
        self.terminal.move_to_row(index)?;
        Ok(())
    }

    fn jump_to_bottom(&mut self, index: u16) -> RenderResult {
        let target_row = self.current_dimensions().rows.saturating_sub(index).saturating_sub(1);
        self.terminal.move_to_row(target_row)?;
        Ok(())
    }

    fn render_text(&mut self, text: &WeightedTextBlock, alignment: &Alignment) -> RenderResult {
        let layout = self.build_layout(alignment.clone());
        let text_drawer = TextDrawer::new(&layout, text, self.current_dimensions(), &self.colors)?;
        text_drawer.draw(self.terminal)
    }

    fn render_line_break(&mut self) -> RenderResult {
        self.terminal.move_to_next_line(1)?;
        Ok(())
    }

    fn render_image(&mut self, image: &Image, properties: &ImageProperties) -> RenderResult {
        let rect = self.current_rect();
        let starting_position = CursorPosition { row: self.terminal.cursor_row, column: rect.start_column };

        let (width, height) = image.dimensions();
        let (cursor_position, columns, rows) = match properties.size {
            ImageSize::Scaled => {
                let scale = scale_image(&rect.dimensions, width, height, &starting_position);
                (CursorPosition { row: starting_position.row, column: scale.start_column }, scale.columns, scale.rows)
            }
            ImageSize::Specific(columns, rows) => (starting_position.clone(), columns, rows),
        };

        let options = PrintOptions {
            columns,
            rows,
            cursor_position,
            z_index: properties.z_index,
            column_width: rect.dimensions.pixels_per_column() as u16,
            row_height: rect.dimensions.pixels_per_row() as u16,
            background_color: properties.background_color,
        };
        self.terminal.print_image(image, &options)?;
        if properties.restore_cursor {
            self.terminal.move_to(starting_position.column, starting_position.row)?;
        } else {
            self.terminal.move_to_row(starting_position.row + rows)?;
        }
        Ok(())
    }

    fn render_preformatted_line(&mut self, operation: &PreformattedLine) -> RenderResult {
        let PreformattedLine { text, unformatted_length, block_length, alignment } = operation;
        let layout = self.build_layout(alignment.clone());

        let dimensions = self.current_dimensions();
        let Positioning { max_line_length, start_column } = layout.compute(dimensions, *block_length);
        if self.options.validate_overflows && unformatted_length > &max_line_length {
            return Err(RenderError::HorizontalOverflow);
        }

        self.terminal.move_to_column(start_column)?;

        // Pad this code block with spaces so we get a nice little rectangle.
        let until_right_edge = max_line_length.saturating_sub(*unformatted_length);
        self.terminal.print_line(text)?;
        self.terminal.print_line(&" ".repeat(until_right_edge as usize))?;

        // If this line is longer than the screen, our cursor wrapped around so we need to update
        // the terminal.
        if *unformatted_length > max_line_length {
            let lines_wrapped = *unformatted_length / max_line_length;
            let new_row = self.terminal.cursor_row + lines_wrapped;
            self.terminal.sync_cursor_row(new_row)?;
        }

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
        let columns = columns.iter().copied().map(u16::from).collect();
        let current_position = self.terminal.cursor_row;
        self.layout = LayoutState::InitializedColumn { columns, start_row: current_position };
        Ok(())
    }

    fn enter_column(&mut self, column_index: usize) -> RenderResult {
        let (columns, start_row) = match mem::take(&mut self.layout) {
            LayoutState::Default => return Err(RenderError::InvalidLayoutEnter),
            LayoutState::InitializedColumn { columns, .. } | LayoutState::EnteredColumn { columns, .. }
                if column_index >= columns.len() =>
            {
                return Err(RenderError::InvalidLayoutEnter);
            }
            LayoutState::InitializedColumn { columns, start_row } => (columns, start_row),
            LayoutState::EnteredColumn { columns, start_row, .. } => {
                // Pop this one and start clean
                self.pop_margin()?;
                (columns, start_row)
            }
        };
        let total_column_units: u16 = columns.iter().sum();
        let column_units_before: u16 = columns.iter().take(column_index).sum();
        let current_rect = self.current_rect();
        let unit_width = current_rect.dimensions.columns as f64 / total_column_units as f64;
        let start_column = current_rect.start_column + (unit_width * column_units_before as f64) as u16;
        let new_column_count = (total_column_units - columns[column_index]) * unit_width as u16;
        let new_size = current_rect.dimensions.shrink_columns(new_column_count);
        let mut dimensions = WindowRect { dimensions: new_size, start_column };
        // Shrink every column's right edge except for last
        if column_index < columns.len() - 1 {
            dimensions = dimensions.shrink_right(4);
        }
        // Shrink every column's left edge except for first
        if column_index > 0 {
            dimensions = dimensions.shrink_left(4);
        }

        self.window_rects.push(dimensions);
        self.layout = LayoutState::EnteredColumn { columns, start_row };
        self.terminal.move_to_row(start_row)?;
        Ok(())
    }

    fn exit_layout(&mut self) -> RenderResult {
        match &self.layout {
            LayoutState::Default | LayoutState::InitializedColumn { .. } => Ok(()),
            LayoutState::EnteredColumn { .. } => {
                self.terminal.move_to(0, self.max_modified_row)?;
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
        columns: Vec<u16>,
        start_row: u16,
    },
    EnteredColumn {
        columns: Vec<u16>,
        start_row: u16,
    },
}

#[derive(Clone, Debug)]
struct WindowRect {
    dimensions: WindowSize,
    start_column: u16,
}

impl WindowRect {
    fn apply_margin(&self, margin: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(margin.saturating_mul(2));
        let start_column = self.start_column + margin;
        Self { dimensions, start_column }
    }

    fn shrink_left(&self, size: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(size);
        let start_column = self.start_column.saturating_add(size);
        Self { dimensions, start_column }
    }

    fn shrink_right(&self, size: u16) -> Self {
        let dimensions = self.dimensions.shrink_columns(size);
        Self { dimensions, start_column: self.start_column }
    }

    fn shrink_rows(&self, rows: u16) -> Self {
        let dimensions = self.dimensions.shrink_rows(rows);
        Self { dimensions, start_column: self.start_column }
    }
}
