use crate::{
    elements::{Element, FormattedText, Text, TextChunk},
    media::{DrawMedia, KittyTerminal},
    resource::Resources,
    slide::Slide,
};
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal::{self, ClearType},
    QueueableCommand,
};
use std::io::{self, Write};

pub struct Drawer {
    handle: io::Stdout,
    resources: Resources,
}

impl Drawer {
    pub fn new() -> io::Result<Self> {
        let mut handle = io::stdout();
        handle.queue(cursor::Hide)?;

        let resources = Resources::default();
        Ok(Self { handle, resources })
    }

    pub fn draw(&mut self, slides: &[Slide]) -> io::Result<()> {
        self.handle.queue(terminal::Clear(ClearType::All))?;
        self.handle.queue(cursor::MoveTo(0, 0))?;

        self.draw_slide(&slides[0])
    }

    fn draw_slide(&mut self, slide: &Slide) -> io::Result<()> {
        for element in &slide.elements {
            self.draw_element(element)?;
        }
        self.handle.flush()?;
        Ok(())
    }

    fn draw_element(&mut self, element: &Element) -> io::Result<()> {
        self.handle.queue(cursor::MoveToColumn(0))?;
        match element {
            // TODO handle level
            Element::Heading { text, .. } => {
                self.handle.queue(style::SetAttribute(style::Attribute::Bold))?;
                self.draw_text(text)?;
                self.handle.queue(cursor::MoveDown(2))?;
                self.handle.queue(style::SetAttribute(style::Attribute::Reset))?;
            }
            Element::Paragraph { text } => {
                self.draw_text(text)?;
                self.handle.queue(cursor::MoveDown(2))?;
            }
        };
        Ok(())
    }

    fn draw_text(&mut self, text: &Text) -> io::Result<()> {
        for chunk in &text.chunks {
            match chunk {
                TextChunk::Formatted(text) => self.draw_formatted_text(text)?,
                TextChunk::Image { url, .. } => self.draw_image(&url)?,
            }
        }
        Ok(())
    }

    fn draw_formatted_text(&mut self, text: &FormattedText) -> io::Result<()> {
        let mut styled = text.text.clone().stylize();
        if text.format.has_bold() {
            styled = styled.bold();
        }
        if text.format.has_italics() {
            styled = styled.italic();
        }
        self.handle.queue(style::PrintStyledContent(styled))?;
        Ok(())
    }

    fn draw_image(&mut self, path: &str) -> io::Result<()> {
        let image = self.resources.image(path)?;
        KittyTerminal.draw_image(&image, &mut self.handle)
    }
}
