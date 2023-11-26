use super::properties::CursorPosition;
use crate::style::Colors;
use crossterm::{
    cursor,
    style::{self, StyledContent},
    terminal::{self},
    QueueableCommand,
};
use std::io;

/// A wrapper over the terminal write handle.
pub(crate) struct Terminal<W>
where
    W: io::Write,
{
    writer: W,
    pub(crate) cursor_row: u16,
}

impl<W: io::Write> Terminal<W> {
    pub(crate) fn new(mut writer: W) -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        writer.queue(cursor::Hide)?;
        writer.queue(terminal::EnterAlternateScreen)?;

        Ok(Self { writer, cursor_row: 0 })
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

    pub(crate) fn sync_cursor_row(&mut self) -> io::Result<()> {
        self.cursor_row = CursorPosition::current()?.row;
        Ok(())
    }

    pub(crate) fn manual_sync_cursor_row(&mut self, position: u16) {
        self.cursor_row = position;
    }
}

impl<W> Drop for Terminal<W>
where
    W: io::Write,
{
    fn drop(&mut self) {
        let _ = self.writer.queue(terminal::LeaveAlternateScreen);
        let _ = self.writer.queue(cursor::Show);
        let _ = self.writer.flush();
        let _ = terminal::disable_raw_mode();
    }
}
