use crate::{
    markdown::elements::{FormattedText, PresentationMetadata, TextFormat},
    markdown::process::{Slide, SlideElement},
    presentation::Presentation,
    render::media::MediaDrawer,
    resource::Resources,
    theme::{Alignment, AuthorPositioning, Colors, ElementType, SlideTheme},
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

    pub fn draw_slide<'a>(
        &mut self,
        resources: &'a mut Resources,
        theme: &'a SlideTheme,
        presentation: &'a Presentation,
    ) -> DrawResult {
        let dimensions = window_size()?;
        let slide_dimensions = WindowSize {
            rows: dimensions.rows - 3,
            columns: dimensions.columns,
            width: dimensions.width,
            height: dimensions.height,
        };

        let slide = presentation.current_slide();
        let slide_drawer = SlideDrawer { handle: &mut self.handle, resources, theme, dimensions: slide_dimensions };
        slide_drawer.draw_slide(slide)?;

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
    resources: &'a mut Resources,
    theme: &'a SlideTheme,
    dimensions: WindowSize,
}

impl<'a, W> SlideDrawer<'a, W>
where
    W: io::Write,
{
    fn draw_slide(mut self, slide: &Slide) -> DrawResult {
        self.apply_theme_colors()?;
        self.handle.queue(terminal::Clear(ClearType::All))?;
        self.handle.queue(cursor::MoveTo(0, 0))?;
        for element in &slide.elements {
            self.apply_theme_colors()?;
            self.draw_element(element)?;
        }
        Ok(())
    }

    fn apply_theme_colors(&mut self) -> io::Result<()> {
        apply_colors(self.handle, &self.theme.styles.default_style.colors)
    }

    fn draw_element(&mut self, element: &SlideElement) -> DrawResult {
        match element {
            SlideElement::PresentationMetadata(meta) => self.draw_presentation_metadata(meta),
            SlideElement::TextLine { texts, element_type } => self.draw_text(texts, element_type),
            SlideElement::Separator => self.draw_separator(),
            SlideElement::LineBreak => self.draw_line_break(),
            SlideElement::Image { url } => self.draw_image(url),
            SlideElement::PreformattedLine { text, original_length, block_length } => {
                self.draw_preformatted_line(text, *original_length, *block_length)
            }
        }
    }

    fn draw_presentation_metadata(&mut self, metadata: &PresentationMetadata) -> DrawResult {
        let center_row = self.dimensions.rows / 2;
        let title = FormattedText::formatted(metadata.title.clone(), TextFormat::default().add_bold());
        let sub_title = metadata.sub_title.as_ref().map(|text| FormattedText::plain(text.clone()));
        let author = metadata.author.as_ref().map(|text| FormattedText::plain(text.clone()));
        self.handle.queue(cursor::MoveToRow(center_row))?;
        self.draw_text(&[title], &ElementType::PresentationTitle)?;
        self.handle.queue(cursor::MoveToNextLine(1))?;
        if let Some(text) = sub_title {
            self.draw_text(&[text], &ElementType::PresentationSubTitle)?;
            self.handle.queue(cursor::MoveToNextLine(1))?;
        }
        if let Some(text) = author {
            match self.theme.styles.presentation.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.handle.queue(cursor::MoveToNextLine(3))?;
                }
                AuthorPositioning::PageBottom => {
                    self.handle.queue(cursor::MoveToRow(self.dimensions.rows))?;
                }
            };
            self.draw_text(&[text], &ElementType::PresentationAuthor)?;
        }
        Ok(())
    }

    fn draw_text(&mut self, text: &[FormattedText], element_type: &ElementType) -> DrawResult {
        if text.is_empty() {
            return Ok(());
        }
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

    fn draw_separator(&mut self) -> DrawResult {
        let separator: String = "—".repeat(self.dimensions.columns as usize);
        self.handle.queue(style::Print(separator))?;
        Ok(())
    }

    fn draw_line_break(&mut self) -> DrawResult {
        self.handle.queue(cursor::MoveToNextLine(1))?;
        Ok(())
    }

    fn draw_image(&mut self, path: &str) -> Result<(), DrawSlideError> {
        let image = self.resources.image(path).map_err(|e| DrawSlideError::Other(Box::new(e)))?;
        MediaDrawer.draw_image(&image, &self.dimensions).map_err(|e| DrawSlideError::Other(Box::new(e)))?;
        Ok(())
    }

    fn draw_preformatted_line(&mut self, text: &str, original_length: usize, block_length: usize) -> DrawResult {
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
    elements: &'a [FormattedText],
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
        elements: &'a [FormattedText],
        dimensions: &WindowSize,
        default_colors: &'a Colors,
    ) -> Self {
        let text_length: u16 = elements.iter().map(|chunk| chunk.text.len() as u16).sum();
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
        Self { handle, elements, start_column, line_length, default_colors }
    }

    fn draw(self, theme: &SlideTheme) -> DrawResult {
        let mut length_so_far = 0;
        self.handle.queue(cursor::MoveToColumn(self.start_column))?;
        for element in self.elements {
            let (mut chunk, mut rest) = self.truncate(&element.text);
            loop {
                let mut styled = chunk.to_string().stylize();
                if element.format.has_bold() {
                    styled = styled.bold();
                }
                if element.format.has_italics() {
                    styled = styled.italic();
                }
                if element.format.has_strikethrough() {
                    styled = styled.crossed_out();
                }
                if element.format.has_code() {
                    styled = styled.italic();
                    if let Some(color) = &theme.styles.code.colors.foreground {
                        styled = styled.with(*color);
                    }
                    if let Some(color) = &theme.styles.code.colors.background {
                        styled = styled.on(*color);
                    }
                }
                length_so_far += styled.content().len() as u16;
                if length_so_far > self.line_length {
                    self.handle.queue(cursor::MoveDown(1))?;
                    self.handle.queue(cursor::MoveToColumn(self.start_column))?;
                }
                self.handle.queue(style::PrintStyledContent(styled))?;
                apply_colors(self.handle, self.default_colors)?;
                if rest.is_empty() {
                    break;
                }
                (chunk, rest) = self.truncate(rest);
            }
        }
        Ok(())
    }

    fn truncate(&self, word: &'a str) -> (&'a str, &'a str) {
        let line_length = self.line_length as usize;
        if word.len() <= line_length {
            return (word, "");
        }
        let target_chunk = &word[0..line_length];
        let output_chunk = match target_chunk.rsplit_once(' ') {
            Some((before, _)) => before,
            None => target_chunk,
        };
        (output_chunk, word[output_chunk.len()..].trim())
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
