use crate::{
    markdown::text_style::{Color, Colors, TextStyle},
    terminal::image::{
        Image,
        printer::{ImagePrinter, PrintImage, PrintImageError, PrintOptions},
    },
};
use crossterm::{
    QueueableCommand, cursor, style,
    terminal::{self},
};
use std::{
    io::{self, Write},
    sync::Arc,
};

pub(crate) trait TerminalIo {
    fn begin_update(&mut self) -> io::Result<()>;
    fn end_update(&mut self) -> io::Result<()>;
    fn cursor_row(&self) -> u16;
    fn move_to(&mut self, column: u16, row: u16) -> io::Result<()>;
    fn move_to_row(&mut self, row: u16) -> io::Result<()>;
    fn move_to_column(&mut self, column: u16) -> io::Result<()>;
    fn move_down(&mut self, amount: u16) -> io::Result<()>;
    fn move_to_next_line(&mut self) -> io::Result<()>;
    fn print_text(&mut self, content: &str, style: &TextStyle, properties: &TextProperties) -> io::Result<()>;
    fn clear_screen(&mut self) -> io::Result<()>;
    fn set_colors(&mut self, colors: Colors) -> io::Result<()>;
    fn set_background_color(&mut self, color: Color) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
    fn print_image(&mut self, image: &Image, options: &PrintOptions) -> Result<(), PrintImageError>;
    fn suspend(&mut self);
    fn resume(&mut self);
}

/// A wrapper over the terminal write handle.
pub(crate) struct Terminal<I: TerminalWrite> {
    writer: I,
    image_printer: Arc<ImagePrinter>,
    cursor_row: u16,
    current_row_height: u16,
}

impl<I: TerminalWrite> Terminal<I> {
    pub(crate) fn new(mut writer: I, image_printer: Arc<ImagePrinter>) -> io::Result<Self> {
        writer.init()?;
        Ok(Self { writer, image_printer, cursor_row: 0, current_row_height: 1 })
    }
}

impl<I: TerminalWrite> TerminalIo for Terminal<I> {
    fn begin_update(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::BeginSynchronizedUpdate)?;
        Ok(())
    }

    fn end_update(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::EndSynchronizedUpdate)?;
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.cursor_row
    }

    fn move_to(&mut self, column: u16, row: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveTo(column, row))?;
        self.cursor_row = row;
        Ok(())
    }

    fn move_to_row(&mut self, row: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveToRow(row))?;
        self.cursor_row = row;
        Ok(())
    }

    fn move_to_column(&mut self, column: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveToColumn(column))?;
        Ok(())
    }

    fn move_down(&mut self, amount: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveDown(amount))?;
        self.cursor_row += amount;
        Ok(())
    }

    fn move_to_next_line(&mut self) -> io::Result<()> {
        let amount = self.current_row_height;
        self.writer.queue(cursor::MoveToNextLine(amount))?;
        self.cursor_row += amount;
        self.current_row_height = 1;
        Ok(())
    }

    fn print_text(&mut self, content: &str, style: &TextStyle, properties: &TextProperties) -> io::Result<()> {
        let content = style.apply(content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.writer.queue(style::PrintStyledContent(content))?;
        self.current_row_height = self.current_row_height.max(properties.height as u16);
        Ok(())
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::Clear(terminal::ClearType::All))?;
        self.cursor_row = 0;
        Ok(())
    }

    fn set_colors(&mut self, colors: Colors) -> io::Result<()> {
        let colors = colors.try_into().map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        self.writer.queue(style::ResetColor)?;
        self.writer.queue(style::SetColors(colors))?;
        Ok(())
    }

    fn set_background_color(&mut self, color: Color) -> io::Result<()> {
        let color = color.try_into().map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        self.writer.queue(style::SetBackgroundColor(color))?;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    fn print_image(&mut self, image: &Image, options: &PrintOptions) -> Result<(), PrintImageError> {
        self.move_to_column(options.cursor_position.column)?;
        self.image_printer.print(&image.image, options, &mut self.writer)?;
        self.cursor_row += options.rows;
        Ok(())
    }

    fn suspend(&mut self) {
        self.writer.deinit();
    }

    fn resume(&mut self) {
        let _ = self.writer.init();
    }
}

impl<I: TerminalWrite> Drop for Terminal<I> {
    fn drop(&mut self) {
        self.writer.deinit();
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TextProperties {
    pub(crate) height: u8,
}

impl Default for TextProperties {
    fn default() -> Self {
        Self { height: 1 }
    }
}

pub(crate) fn should_hide_cursor() -> bool {
    // WezTerm on Windows fails to display images if we've hidden the cursor so we **always** hide it
    // unless we're on WezTerm on Windows.
    let term = std::env::var("TERM_PROGRAM");
    let is_wezterm = term.as_ref().map(|s| s.as_str()) == Ok("WezTerm");
    !(is_windows_based_os() && is_wezterm)
}

fn is_windows_based_os() -> bool {
    let is_windows = std::env::consts::OS == "windows";
    let is_wsl = std::env::var("WSL_DISTRO_NAME").is_ok();
    is_windows || is_wsl
}

pub(crate) trait TerminalWrite: io::Write {
    fn init(&mut self) -> io::Result<()>;
    fn deinit(&mut self);
}

impl TerminalWrite for io::Stdout {
    fn init(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        if should_hide_cursor() {
            self.queue(cursor::Hide)?;
        }
        self.queue(terminal::EnterAlternateScreen)?;
        Ok(())
    }

    fn deinit(&mut self) {
        let _ = self.queue(terminal::LeaveAlternateScreen);
        if should_hide_cursor() {
            let _ = self.queue(cursor::Show);
        }
        let _ = self.flush();
        let _ = terminal::disable_raw_mode();
    }
}
