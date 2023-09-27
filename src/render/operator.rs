use super::{
    draw::{DrawResult, DrawSlideError},
    layout::Layout,
    media::{Image, MediaDrawer},
    text::TextDrawer,
};
use crate::{
    markdown::text::WeightedLine,
    presentation::{AsRenderOperations, RenderOperation},
    render::{layout::Positioning, properties::WindowSize},
    theme::{Alignment, Colors},
};
use crossterm::{
    cursor, style,
    terminal::{self, ClearType},
    QueueableCommand,
};
use std::io;

pub(crate) struct RenderOperator<'a, W> {
    handle: &'a mut W,
    slide_dimensions: WindowSize,
    window_dimensions: WindowSize,
    colors: Colors,
}

impl<'a, W> RenderOperator<'a, W>
where
    W: io::Write,
{
    pub(crate) fn new(
        handle: &'a mut W,
        slide_dimensions: WindowSize,
        window_dimensions: WindowSize,
        colors: Colors,
    ) -> Self {
        Self { handle, slide_dimensions, window_dimensions, colors }
    }

    pub(crate) fn render(&mut self, operation: &RenderOperation) -> DrawResult {
        match operation {
            RenderOperation::ClearScreen => self.clear_screen(),
            RenderOperation::SetColors(colors) => self.set_colors(colors),
            RenderOperation::JumpToVerticalCenter => self.jump_to_vertical_center(),
            RenderOperation::JumpToSlideBottom => self.jump_to_slide_bottom(),
            RenderOperation::JumpToWindowBottom => self.jump_to_window_bottom(),
            RenderOperation::RenderTextLine { texts, alignment } => self.render_text(texts, alignment),
            RenderOperation::RenderSeparator => self.render_separator(),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image) => self.render_image(image),
            RenderOperation::RenderPreformattedLine { text, unformatted_length, block_length, alignment } => {
                self.render_preformatted_line(text, *unformatted_length, *block_length, alignment)
            }
            RenderOperation::RenderDynamic(generator) => self.render_dynamic(generator.as_ref()),
        }
    }

    fn clear_screen(&mut self) -> DrawResult {
        self.handle.queue(terminal::Clear(ClearType::All))?;
        self.handle.queue(cursor::MoveTo(0, 0))?;
        Ok(())
    }

    fn set_colors(&mut self, colors: &Colors) -> DrawResult {
        self.colors = colors.clone();
        self.apply_colors()
    }

    fn apply_colors(&mut self) -> DrawResult {
        self.handle.queue(style::SetColors(style::Colors {
            background: self.colors.background,
            foreground: self.colors.foreground,
        }))?;
        Ok(())
    }

    fn jump_to_vertical_center(&mut self) -> DrawResult {
        let center_row = self.slide_dimensions.rows / 2;
        self.handle.queue(cursor::MoveToRow(center_row))?;
        Ok(())
    }

    fn jump_to_slide_bottom(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToRow(self.slide_dimensions.rows))?;
        Ok(())
    }

    fn jump_to_window_bottom(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToRow(self.window_dimensions.rows))?;
        Ok(())
    }

    fn render_text(&mut self, text: &WeightedLine, alignment: &Alignment) -> DrawResult {
        let text_drawer = TextDrawer::new(alignment, &mut self.handle, text, &self.slide_dimensions, &self.colors);
        text_drawer.draw()
    }

    fn render_separator(&mut self) -> DrawResult {
        let separator: String = "—".repeat(self.slide_dimensions.columns as usize);
        self.handle.queue(style::Print(separator))?;
        Ok(())
    }

    fn render_line_break(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToNextLine(1))?;
        Ok(())
    }

    fn render_image(&mut self, image: &Image) -> DrawResult {
        MediaDrawer.draw_image(image, &self.slide_dimensions).map_err(|e| DrawSlideError::Other(Box::new(e)))?;
        Ok(())
    }

    fn render_preformatted_line(
        &mut self,
        text: &str,
        unformatted_length: usize,
        block_length: usize,
        alignment: &Alignment,
    ) -> DrawResult {
        let Positioning { max_line_length, start_column } =
            Layout(alignment).compute(&self.slide_dimensions, block_length as u16);
        self.handle.queue(cursor::MoveToColumn(start_column))?;

        let until_right_edge = usize::from(max_line_length).saturating_sub(unformatted_length);

        // Pad this code block with spaces so we get a nice little rectangle.
        self.handle.queue(style::Print(&text))?;
        self.handle.queue(style::Print(" ".repeat(until_right_edge)))?;

        // Restore colors
        self.apply_colors()?;
        Ok(())
    }

    fn render_dynamic(&mut self, generator: &dyn AsRenderOperations) -> DrawResult {
        let operations = generator.as_render_operations(&self.slide_dimensions);
        for operation in operations {
            self.render(&operation)?;
        }
        Ok(())
    }
}
