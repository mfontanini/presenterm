use super::media::Image;
use crate::{
    format::TextFormat,
    markdown::text::WeightedLine,
    presentation::{Presentation, RenderOperation, Slide},
    render::media::MediaDrawer,
    theme::{Alignment, Colors, PresentationTheme},
};
use crossterm::{
    cursor, style,
    terminal::{self, disable_raw_mode, enable_raw_mode, window_size, ClearType, WindowSize},
    QueueableCommand,
};
use std::io;

pub type DrawResult = Result<(), DrawSlideError>;

pub struct Drawer<W: io::Write> {
    handle: W,
}

impl<W> Drawer<W>
where
    W: io::Write,
{
    pub fn new(mut handle: W) -> io::Result<Self> {
        enable_raw_mode()?;
        handle.queue(cursor::Hide)?;
        Ok(Self { handle })
    }

    pub fn render_slide<'a>(&mut self, theme: &'a PresentationTheme, presentation: &'a Presentation) -> DrawResult {
        let dimensions = window_size()?;
        let slide_dimensions = WindowSize {
            rows: dimensions.rows - 3,
            columns: dimensions.columns,
            width: dimensions.width,
            height: dimensions.height,
        };

        let slide = presentation.current_slide();
        let slide_drawer =
            SlideDrawer { handle: &mut self.handle, dimensions: slide_dimensions, colors: Default::default() };
        slide_drawer.render_slide(slide)?;

        let rendered_footer = theme.styles.footer.render(
            presentation.current_slide_index(),
            presentation.total_slides(),
            dimensions.columns as usize,
        );
        if let Some(footer) = rendered_footer {
            self.handle.queue(cursor::MoveTo(0, dimensions.rows - 1))?;
            self.handle.queue(style::Print(footer))?;
        }
        self.handle.flush()?;
        Ok(())
    }
}

impl<W> Drop for Drawer<W>
where
    W: io::Write,
{
    fn drop(&mut self) {
        let _ = self.handle.queue(cursor::Show);
        let _ = disable_raw_mode();
    }
}

struct SlideDrawer<'a, W> {
    handle: &'a mut W,
    dimensions: WindowSize,
    colors: Colors,
}

impl<'a, W> SlideDrawer<'a, W>
where
    W: io::Write,
{
    fn render_slide(mut self, slide: &Slide) -> DrawResult {
        for operation in &slide.render_operations {
            self.render(operation)?;
        }
        Ok(())
    }

    fn render(&mut self, operation: &RenderOperation) -> DrawResult {
        match operation {
            RenderOperation::ClearScreen => self.clear_screen(),
            RenderOperation::SetColors(colors) => self.set_colors(colors),
            RenderOperation::JumpToVerticalCenter => self.jump_to_vertical_center(),
            RenderOperation::JumpToBottom => self.jump_to_bottom(),
            RenderOperation::RenderTextLine { texts, alignment } => self.render_text(texts, alignment),
            RenderOperation::RenderSeparator => self.render_separator(),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image) => self.render_image(image),
            RenderOperation::RenderPreformattedLine { text, unformatted_length, block_length, alignment } => {
                self.render_preformatted_line(text, *unformatted_length, *block_length, alignment)
            }
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
        apply_colors(self.handle, &self.colors)?;
        Ok(())
    }

    fn jump_to_vertical_center(&mut self) -> DrawResult {
        let center_row = self.dimensions.rows / 2;
        self.handle.queue(cursor::MoveToRow(center_row))?;
        Ok(())
    }

    fn jump_to_bottom(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToRow(self.dimensions.rows))?;
        Ok(())
    }

    fn render_text(&mut self, text: &WeightedLine, alignment: &Alignment) -> DrawResult {
        let text_drawer = TextDrawer::new(alignment, &mut self.handle, text, &self.dimensions, &self.colors);
        text_drawer.draw()
    }

    fn render_separator(&mut self) -> DrawResult {
        let separator: String = "—".repeat(self.dimensions.columns as usize);
        self.handle.queue(style::Print(separator))?;
        Ok(())
    }

    fn render_line_break(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToNextLine(1))?;
        Ok(())
    }

    fn render_image(&mut self, image: &Image) -> Result<(), DrawSlideError> {
        MediaDrawer.draw_image(image, &self.dimensions).map_err(|e| DrawSlideError::Other(Box::new(e)))?;
        Ok(())
    }

    fn render_preformatted_line(
        &mut self,
        text: &str,
        unformatted_length: usize,
        block_length: usize,
        alignment: &Alignment,
    ) -> DrawResult {
        let start_column = match *alignment {
            Alignment::Left { margin } => margin,
            Alignment::Center { minimum_margin, minimum_size } => {
                let max_line_length = block_length.max(minimum_size as usize);
                let column = (self.dimensions.columns - max_line_length as u16) / 2;
                column.max(minimum_margin)
            }
        };
        self.handle.queue(cursor::MoveToColumn(start_column))?;

        let max_line_length = (self.dimensions.columns - start_column * 2) as usize;
        let until_right_edge = max_line_length.saturating_sub(unformatted_length);

        // Pad this code block with spaces so we get a nice little rectangle.
        self.handle.queue(style::Print(&text))?;
        self.handle.queue(style::Print(" ".repeat(until_right_edge)))?;

        // Restore colors
        self.apply_colors()?;
        Ok(())
    }
}

struct TextDrawer<'a, W> {
    handle: &'a mut W,
    line: &'a WeightedLine,
    start_column: u16,
    line_length: u16,
    default_colors: &'a Colors,
}

impl<'a, W> TextDrawer<'a, W>
where
    W: io::Write,
{
    fn new(
        alignment: &'a Alignment,
        handle: &'a mut W,
        line: &'a WeightedLine,
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Self {
        let text_length = line.width() as u16;
        let mut line_length = dimensions.columns;
        let mut start_column;
        match *alignment {
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
        Self { handle, line, start_column, line_length, default_colors }
    }

    fn draw(self) -> DrawResult {
        self.handle.queue(cursor::MoveToColumn(self.start_column))?;

        for (line_index, line) in self.line.split(self.line_length as usize).enumerate() {
            self.handle.queue(cursor::MoveToColumn(self.start_column))?;
            if line_index > 0 {
                self.handle.queue(cursor::MoveDown(1))?;
            }
            for chunk in line {
                let (text, format) = chunk.into_parts();
                let text = format.apply(text);
                self.handle.queue(style::PrintStyledContent(text))?;

                // Crossterm resets colors if any attributes are set so let's just re-apply colors
                // if the format has anything on it at all.
                if format != TextFormat::default() {
                    apply_colors(self.handle, self.default_colors)?;
                }
            }
        }
        Ok(())
    }
}

fn apply_colors<W: io::Write>(handle: &mut W, colors: &Colors) -> io::Result<()> {
    handle.queue(style::SetColors(style::Colors { background: colors.background, foreground: colors.foreground }))?;
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum DrawSlideError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("unsupported structure: {0}")]
    UnsupportedStructure(&'static str),

    #[error(transparent)]
    Other(Box<dyn std::error::Error>),
}
