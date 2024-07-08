use crate::{
    media::{
        image::Image,
        printer::{ImagePrinter, PrintImage, PrintImageError, PrintOptions},
    },
    style::Colors,
};
use crossterm::{
    cursor,
    style::{self, StyledContent},
    terminal::{self},
    QueueableCommand,
};
use std::{
    io::{self, Write},
    sync::Arc,
};

/// A wrapper over the terminal write handle.
pub(crate) struct Terminal<W>
where
    W: TerminalWrite,
{
    writer: W,
    image_printer: Arc<ImagePrinter>,
    pub(crate) cursor_row: u16,
}

impl<W: TerminalWrite> Terminal<W> {
    pub(crate) fn new(mut writer: W, image_printer: Arc<ImagePrinter>) -> io::Result<Self> {
        writer.init()?;
        Ok(Self { writer, image_printer, cursor_row: 0 })
    }

    pub(crate) fn begin_update(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::BeginSynchronizedUpdate)?;
        Ok(())
    }

    pub(crate) fn end_update(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::EndSynchronizedUpdate)?;
        Ok(())
    }

    pub(crate) fn move_to(&mut self, column: u16, row: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveTo(column, row))?;
        self.cursor_row = row;
        Ok(())
    }

    pub(crate) fn move_to_row(&mut self, row: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveToRow(row))?;
        self.cursor_row = row;
        Ok(())
    }

    pub(crate) fn move_to_column(&mut self, column: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveToColumn(column))?;
        Ok(())
    }

    pub(crate) fn move_down(&mut self, amount: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveDown(amount))?;
        self.cursor_row += amount;
        Ok(())
    }

    pub(crate) fn move_to_next_line(&mut self, amount: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveToNextLine(amount))?;
        self.cursor_row += amount;
        Ok(())
    }

    pub(crate) fn print_line(&mut self, text: &str) -> io::Result<()> {
        self.writer.queue(style::Print(text))?;
        Ok(())
    }

    pub(crate) fn print_styled_line(&mut self, content: StyledContent<String>) -> io::Result<()> {
        self.writer.queue(style::PrintStyledContent(content))?;
        Ok(())
    }

    pub(crate) fn clear_screen(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::Clear(terminal::ClearType::All))?;
        self.cursor_row = 0;
        Ok(())
    }

    pub(crate) fn set_colors(&mut self, colors: Colors) -> io::Result<()> {
        self.writer.queue(style::ResetColor)?;
        self.writer.queue(style::SetColors(colors.into()))?;
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    pub(crate) fn sync_cursor_row(&mut self, position: u16) -> io::Result<()> {
        self.cursor_row = position;
        self.writer.queue(cursor::MoveToRow(position))?;
        Ok(())
    }

    pub(crate) fn print_image(&mut self, image: &Image, options: &PrintOptions) -> Result<(), PrintImageError> {
        self.move_to_column(options.cursor_position.column)?;
        self.image_printer.print(&image.resource, options, &mut self.writer)?;
        self.cursor_row += options.rows;
        Ok(())
    }
}

impl<W> Drop for Terminal<W>
where
    W: TerminalWrite,
{
    fn drop(&mut self) {
        self.writer.deinit();
    }
}

fn should_hide_cursor() -> bool {
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

pub trait TerminalWrite: io::Write {
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
