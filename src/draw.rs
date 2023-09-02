use crate::{
    elements::{Code, Element, FormattedText, ListItem, ListItemType, Text, TextChunk},
    highlighting::CodeHighlighter,
    media::{DrawMedia, KittyTerminal},
    resource::Resources,
    slide::Slide,
};
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal::{self, window_size, ClearType, WindowSize},
    QueueableCommand,
};
use std::{io, iter};

pub struct Drawer<'a, W> {
    handle: &'a mut W,
    resources: &'a mut Resources,
    highlighter: &'a CodeHighlighter,
    dimensions: WindowSize,
}

impl<'a, W> Drawer<'a, W>
where
    W: io::Write,
{
    pub fn new(handle: &'a mut W, resources: &'a mut Resources, highlighter: &'a CodeHighlighter) -> io::Result<Self> {
        let dimensions = window_size()?;
        Ok(Self { handle, resources, highlighter, dimensions })
    }

    pub fn draw_slide(mut self, slide: &Slide) -> io::Result<()> {
        self.handle.queue(cursor::Hide)?;
        self.handle.queue(terminal::Clear(ClearType::All))?;
        self.handle.queue(cursor::MoveTo(0, 0))?;
        for element in &slide.elements {
            self.draw_element(element)?;
        }
        self.handle.flush()?;
        Ok(())
    }

    fn draw_element(&mut self, element: &Element) -> io::Result<()> {
        self.handle.queue(cursor::MoveToColumn(0))?;
        match element {
            Element::SlideTitle { text } => self.draw_slide_title(text),
            Element::Heading { text, level } => self.draw_heading(text, *level),
            Element::Paragraph(text) => self.draw_paragraph(text),
            Element::List(items) => self.draw_list(items),
            Element::Code(code) => self.draw_code(code),
        }
    }

    fn draw_slide_title(&mut self, text: &Text) -> io::Result<()> {
        self.handle.queue(cursor::MoveDown(1))?;
        self.handle.queue(style::SetAttribute(style::Attribute::Bold))?;
        self.draw_text(text)?;
        self.handle.queue(style::SetAttribute(style::Attribute::Reset))?;
        self.handle.queue(cursor::MoveDown(2))?;
        self.handle.queue(cursor::MoveToColumn(0))?;

        let separator: String = iter::repeat('—').take(self.dimensions.columns as usize).collect();
        self.handle.queue(style::Print(separator))?;
        self.handle.queue(cursor::MoveDown(2))?;
        Ok(())
    }

    fn draw_heading(&mut self, text: &Text, _level: u8) -> io::Result<()> {
        // TODO handle level
        self.handle.queue(style::SetAttribute(style::Attribute::Bold))?;
        self.draw_text(text)?;
        self.handle.queue(style::SetAttribute(style::Attribute::Reset))?;
        self.handle.queue(cursor::MoveDown(2))?;
        Ok(())
    }

    fn draw_paragraph(&mut self, text: &Text) -> io::Result<()> {
        self.draw_text(text)?;
        self.handle.queue(cursor::MoveDown(2))?;
        Ok(())
    }

    fn draw_text(&mut self, text: &Text) -> io::Result<()> {
        for chunk in &text.chunks {
            match chunk {
                TextChunk::Formatted(text) => self.draw_formatted_text(text)?,
                TextChunk::Image { url, .. } => self.draw_image(&url)?,
                TextChunk::LineBreak => {
                    self.handle.queue(cursor::MoveDown(1))?;
                    self.handle.queue(cursor::MoveToColumn(0))?;
                }
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

    fn draw_list(&mut self, items: &[ListItem]) -> io::Result<()> {
        for item in items {
            self.draw_list_item(item)?;
        }
        self.handle.queue(cursor::MoveDown(2))?;
        Ok(())
    }

    fn draw_list_item(&mut self, item: &ListItem) -> io::Result<()> {
        let padding_length = (item.depth as usize + 1) * 2;
        let padding: String = std::iter::repeat(' ').take(padding_length).collect();
        self.handle.queue(cursor::MoveToColumn(0))?;
        self.handle.queue(style::Print(padding))?;
        match item.item_type {
            ListItemType::Unordered => {
                let delimiter = match item.depth {
                    0 => '•',
                    1 => '◦',
                    _ => '▪',
                };
                self.handle.queue(style::Print(delimiter))?;
            }
            ListItemType::OrderedParens(number) => {
                self.handle.queue(style::Print(number))?;
                self.handle.queue(style::Print(") "))?;
            }
            ListItemType::OrderedPeriod(number) => {
                self.handle.queue(style::Print(number))?;
                self.handle.queue(style::Print(". "))?;
            }
        };
        self.handle.queue(style::Print(" "))?;
        self.draw_text(&item.contents)?;
        self.handle.queue(cursor::MoveDown(1))?;
        Ok(())
    }

    fn draw_code(&mut self, code: &Code) -> io::Result<()> {
        for line in self.highlighter.highlight(&code.contents, &code.language) {
            self.handle.queue(style::Print(line))?;
        }
        Ok(())
    }
}
