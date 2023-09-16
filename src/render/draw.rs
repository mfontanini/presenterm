use super::media::Image;
use crate::{
    markdown::text::WeightedLine,
    presentation::{Presentation, RenderOperation, Slide},
    render::media::MediaDrawer,
    theme::{Alignment, Colors, ElementType, SlideTheme},
};
use crossterm::{
    cursor,
    style::{self, Stylize},
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

    pub fn render_slide<'a>(&mut self, theme: &'a SlideTheme, presentation: &'a Presentation) -> DrawResult {
        let dimensions = window_size()?;
        let slide_dimensions = WindowSize {
            rows: dimensions.rows - 3,
            columns: dimensions.columns,
            width: dimensions.width,
            height: dimensions.height,
        };

        let slide = presentation.current_slide();
        let slide_drawer = SlideDrawer { handle: &mut self.handle, theme, dimensions: slide_dimensions };
        slide_drawer.render_slide(slide)?;

        if let Some(template) = &theme.styles.footer.template {
            let current_slide = (presentation.current_slide_index() + 1).to_string();
            let total_slides = presentation.total_slides().to_string();
            let footer = template.replace("{current_slide}", &current_slide).replace("{total_slides}", &total_slides);
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
    theme: &'a SlideTheme,
    dimensions: WindowSize,
}

impl<'a, W> SlideDrawer<'a, W>
where
    W: io::Write,
{
    fn render_slide(mut self, slide: &Slide) -> DrawResult {
        self.apply_theme_colors()?;
        self.handle.queue(terminal::Clear(ClearType::All))?;
        self.handle.queue(cursor::MoveTo(0, 0))?;
        for operation in &slide.render_operations {
            self.apply_theme_colors()?;
            self.render(operation)?;
        }
        Ok(())
    }

    fn apply_theme_colors(&mut self) -> io::Result<()> {
        apply_colors(self.handle, &self.theme.styles.default_style.colors)
    }

    fn render(&mut self, operation: &RenderOperation) -> DrawResult {
        match operation {
            RenderOperation::JumpToVerticalCenter => self.jump_to_vertical_center(),
            RenderOperation::JumpToBottom => self.jump_to_bottom(),
            RenderOperation::RenderTextLine { texts, element_type } => self.render_text(texts, element_type),
            RenderOperation::RenderSeparator => self.render_separator(),
            RenderOperation::RenderLineBreak => self.render_line_break(),
            RenderOperation::RenderImage(image) => self.render_image(image),
            RenderOperation::RenderPreformattedLine { text, original_length, block_length } => {
                self.render_preformatted_line(text, *original_length, *block_length)
            }
        }
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

    fn render_text(&mut self, text: &WeightedLine, element_type: &ElementType) -> DrawResult {
        let alignment = self.theme.alignment(element_type);
        let text_drawer = TextDrawer::new(
            alignment,
            &mut self.handle,
            text,
            &self.dimensions,
            &self.theme.styles.default_style.colors,
        );
        text_drawer.draw(self.theme)
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

    fn render_preformatted_line(&mut self, text: &str, original_length: usize, block_length: usize) -> DrawResult {
        let style = self.theme.alignment(&ElementType::Code);
        let start_column = match *style {
            Alignment::Left { margin } => margin,
            Alignment::Center { minimum_margin, minimum_size } => {
                let max_line_length = block_length.max(minimum_size as usize);
                let column = (self.dimensions.columns - max_line_length as u16) / 2;
                column.max(minimum_margin)
            }
        };
        self.handle.queue(cursor::MoveToColumn(start_column))?;

        let max_line_length = (self.dimensions.columns - start_column * 2) as usize;
        let until_right_edge = max_line_length.saturating_sub(original_length);
        // Pad this code block with spaces so we get a nice little rectangle.
        self.handle.queue(style::Print(&text))?;
        self.handle.queue(style::Print(" ".repeat(until_right_edge)))?;
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

    fn draw(self, theme: &SlideTheme) -> DrawResult {
        self.handle.queue(cursor::MoveToColumn(self.start_column))?;

        for (line_index, line) in self.line.split(self.line_length as usize).enumerate() {
            self.handle.queue(cursor::MoveToColumn(self.start_column))?;
            if line_index > 0 {
                self.handle.queue(cursor::MoveDown(1))?;
            }
            for chunk in line {
                let (text, format) = chunk.into_parts();
                let mut styled = text.to_string().stylize();
                if format.has_bold() {
                    styled = styled.bold();
                }
                if format.has_italics() {
                    styled = styled.italic();
                }
                if format.has_strikethrough() {
                    styled = styled.crossed_out();
                }
                if format.has_code() {
                    styled = styled.italic();
                    if let Some(color) = &theme.styles.code.colors.foreground {
                        styled = styled.with(*color);
                    }
                    if let Some(color) = &theme.styles.code.colors.background {
                        styled = styled.on(*color);
                    }
                }
                self.handle.queue(style::PrintStyledContent(styled))?;
                apply_colors(self.handle, self.default_colors)?;
            }
        }
        Ok(())
    }
}

fn apply_colors<W: io::Write>(handle: &mut W, colors: &Colors) -> io::Result<()> {
    if let Some(color) = colors.background {
        handle.queue(style::SetBackgroundColor(color))?;
    }
    if let Some(color) = colors.foreground {
        handle.queue(style::SetForegroundColor(color))?;
    }
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
