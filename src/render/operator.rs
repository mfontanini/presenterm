use super::{
    draw::{RenderError, RenderResult},
    layout::Layout,
    media::{Image, MediaRender},
    properties::CursorPosition,
    terminal::Terminal,
    text::TextDrawer,
};
use crate::{
    markdown::text::WeightedLine,
    presentation::{AsRenderOperations, PreformattedLine, RenderOperation},
    render::{layout::Positioning, properties::WindowSize},
    style::Colors,
    theme::Alignment,
};
use std::{io, mem};

pub(crate) struct RenderOperator<'a, W>
where
    W: io::Write,
{
    terminal: &'a mut Terminal<W>,
    default_slide_dimensions: WindowSize,
    slide_dimensions: WindowSize,
    window_dimensions: WindowSize,
    colors: Colors,
    max_modified_row: u16,
    layout: LayoutState,
}

impl<'a, W> RenderOperator<'a, W>
where
    W: io::Write,
{
    pub(crate) fn new(
        terminal: &'a mut Terminal<W>,
        slide_dimensions: WindowSize,
        window_dimensions: WindowSize,
    ) -> Self {
        let default_slide_dimensions = slide_dimensions.clone();
        let lowest_modified_row = terminal.cursor_row;
        Self {
            terminal,
            default_slide_dimensions,
            slide_dimensions,
            window_dimensions,
            colors: Default::default(),
            max_modified_row: lowest_modified_row,
            layout: Default::default(),
        }
    }

    pub(crate) fn render(mut self, operations: &[RenderOperation]) -> RenderResult {
        for operation in operations {
            self.render_one(operation)?;
        }
        Ok(())
    }

    fn render_one(&mut self, operation: &RenderOperation) -> RenderResult {
        match operation {
            RenderOperation::ClearScreen => self.clear_screen(),
            RenderOperation::SetColors(colors) => self.set_colors(colors),
            RenderOperation::JumpToVerticalCenter => self.jump_to_vertical_center(),
            RenderOperation::JumpToSlideBottom => self.jump_to_slide_bottom(),
            RenderOperation::JumpToWindowBottom => self.jump_to_window_bottom(),
            RenderOperation::RenderTextLine { line: texts, alignment } => self.render_text(texts, alignment),
            RenderOperation::RenderSeparator => self.render_separator(),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image) => self.render_image(image),
            RenderOperation::RenderPreformattedLine(operation) => self.render_preformatted_line(operation),
            RenderOperation::RenderDynamic(generator) => self.render_dynamic(generator.as_ref()),
            RenderOperation::InitColumnLayout { columns } => self.init_column_layout(columns),
            RenderOperation::EnterColumn { column } => self.enter_column(*column),
            RenderOperation::ExitLayout => self.exit_layout(),
        }?;
        self.max_modified_row = self.max_modified_row.max(self.terminal.cursor_row);
        Ok(())
    }

    fn clear_screen(&mut self) -> RenderResult {
        self.terminal.clear_screen()?;
        self.terminal.move_to(0, 0)?;
        self.max_modified_row = 0;
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
        let center_row = self.slide_dimensions.rows / 2;
        self.terminal.move_to_row(center_row)?;
        Ok(())
    }

    fn jump_to_slide_bottom(&mut self) -> RenderResult {
        self.terminal.move_to_row(self.slide_dimensions.rows)?;
        Ok(())
    }

    fn jump_to_window_bottom(&mut self) -> RenderResult {
        self.terminal.move_to_row(self.window_dimensions.rows)?;
        Ok(())
    }

    fn render_text(&mut self, text: &WeightedLine, alignment: &Alignment) -> RenderResult {
        let layout = self.build_layout(alignment.clone());
        let text_drawer = TextDrawer::new(&layout, text, &self.slide_dimensions, &self.colors)?;
        text_drawer.draw(self.terminal)
    }

    fn render_separator(&mut self) -> RenderResult {
        let separator: String = "—".repeat(self.slide_dimensions.columns as usize);
        if let LayoutState::EnteredColumn { start_column, .. } = &self.layout {
            self.terminal.move_to_column(*start_column)?;
        }
        self.terminal.print_line(&separator)?;
        Ok(())
    }

    fn render_line_break(&mut self) -> RenderResult {
        self.terminal.move_to_next_line(1)?;
        Ok(())
    }

    fn render_image(&mut self, image: &Image) -> RenderResult {
        let mut position = CursorPosition { row: self.terminal.cursor_row, column: 0 };
        if let LayoutState::EnteredColumn { start_column, .. } = &self.layout {
            position.column = *start_column;
        }
        MediaRender.draw_image(image, position, &self.slide_dimensions).map_err(|e| RenderError::Other(Box::new(e)))?;
        // TODO try to avoid
        self.terminal.sync_cursor_row()?;
        Ok(())
    }

    fn render_preformatted_line(&mut self, operation: &PreformattedLine) -> RenderResult {
        let PreformattedLine { text, unformatted_length, block_length, alignment } = operation;
        let layout = self.build_layout(alignment.clone());

        let Positioning { max_line_length, start_column } =
            layout.compute(&self.slide_dimensions, *block_length as u16);
        self.terminal.move_to_column(start_column)?;

        let until_right_edge = usize::from(max_line_length).saturating_sub(*unformatted_length);

        // Pad this code block with spaces so we get a nice little rectangle.
        self.terminal.print_line(text)?;
        self.terminal.print_line(&" ".repeat(until_right_edge))?;

        // Restore colors
        self.apply_colors()?;
        Ok(())
    }

    fn render_dynamic(&mut self, generator: &dyn AsRenderOperations) -> RenderResult {
        let operations = generator.as_render_operations(&self.slide_dimensions);
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
        let current_position = CursorPosition::current()?;
        self.layout = LayoutState::InitializedColumn { columns, start_row: current_position.row };
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
            LayoutState::EnteredColumn { columns, start_row, .. } => (columns, start_row),
        };
        let total_column_units: u16 = columns.iter().sum();
        let column_units_before: u16 = columns.iter().take(column_index).sum();
        let unit_width = self.default_slide_dimensions.columns as f64 / total_column_units as f64;
        let start_column = (unit_width * column_units_before as f64) as u16;
        let new_column_count = (total_column_units - columns[column_index]) * unit_width as u16;
        let new_size = self.default_slide_dimensions.shrink_columns(new_column_count);
        self.slide_dimensions = new_size;
        self.layout = LayoutState::EnteredColumn { columns, start_column, start_row };
        self.terminal.move_to_row(start_row)?;
        Ok(())
    }

    fn exit_layout(&mut self) -> RenderResult {
        self.slide_dimensions = self.default_slide_dimensions.clone();
        match &self.layout {
            LayoutState::Default | LayoutState::InitializedColumn { .. } => Ok(()),
            LayoutState::EnteredColumn { .. } => {
                self.terminal.move_to(0, self.max_modified_row)?;
                self.layout = LayoutState::Default;
                Ok(())
            }
        }
    }

    fn build_layout(&self, alignment: Alignment) -> Layout {
        let mut layout = Layout::new(alignment);
        if let LayoutState::EnteredColumn { start_column: starting_column, .. } = &self.layout {
            layout = layout.with_start_column(*starting_column);
        }
        layout
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
        start_column: u16,
        start_row: u16,
    },
}
