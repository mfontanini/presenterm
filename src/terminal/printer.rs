use super::emulator::TerminalEmulator;
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

#[derive(Debug, PartialEq)]
pub(crate) enum TerminalCommand<'a> {
    BeginUpdate,
    EndUpdate,
    MoveTo { column: u16, row: u16 },
    MoveToRow(u16),
    MoveToColumn(u16),
    MoveDown(u16),
    MoveRight(u16),
    MoveLeft(u16),
    MoveToNextLine,
    PrintText { content: &'a str, style: TextStyle },
    ClearScreen,
    SetColors(Colors),
    SetBackgroundColor(Color),
    SetCursorBoundaries { rows: u16 },
    Flush,
    PrintImage { image: Image, options: PrintOptions },
}

pub(crate) trait TerminalIo {
    fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError>;
    fn cursor_row(&self) -> u16;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TerminalError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("image: {0}")]
    Image(#[from] PrintImageError),
}

/// A wrapper over the terminal write handle.
pub(crate) struct Terminal<I: TerminalWrite> {
    writer: I,
    image_printer: Arc<ImagePrinter>,
    cursor_row: u16,
    current_row_height: u16,
    rows: u16,
    last_cleared_background_color: Option<Color>,
    background_color: Option<Color>,
    osc11_background: bool,
    terminal_progress_enabled: bool,
    terminal_progress: u8,
}

impl<I: TerminalWrite> Terminal<I> {
    pub(crate) fn new(
        mut writer: I,
        image_printer: Arc<ImagePrinter>,
        terminal_progress_enabled: bool,
    ) -> io::Result<Self> {
        writer.init()?;
        Ok(Self {
            writer,
            image_printer,
            cursor_row: 0,
            current_row_height: 1,
            rows: u16::MAX,
            last_cleared_background_color: None,
            background_color: None,
            // Only use OSC11 when outside of tmux temporarily since it somehow breaks under kitty
            osc11_background: !TerminalEmulator::capabilities().tmux,
            terminal_progress_enabled,
            terminal_progress: 0,
        })
    }

    fn begin_update(&mut self) -> io::Result<()> {
        self.writer.queue(terminal::BeginSynchronizedUpdate)?;
        Ok(())
    }

    fn end_update(&mut self) -> io::Result<()> {
        let image_printer = self.image_printer.clone();
        if let ImagePrinter::Kitty(printer) = image_printer.as_ref() {
            printer.end_redraw(&mut self.writer)?;
        }
        self.writer.queue(terminal::EndSynchronizedUpdate)?;
        Ok(())
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

    fn move_right(&mut self, amount: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveRight(amount))?;
        Ok(())
    }

    fn move_left(&mut self, amount: u16) -> io::Result<()> {
        self.writer.queue(cursor::MoveLeft(amount))?;
        Ok(())
    }

    fn move_to_next_line(&mut self) -> io::Result<()> {
        let amount = self.current_row_height;
        self.writer.queue(cursor::MoveToNextLine(amount))?;
        self.cursor_row += amount;
        self.current_row_height = 1;
        Ok(())
    }

    fn print_text(&mut self, content: &str, style: &TextStyle) -> io::Result<()> {
        // Don't print text if it overflows vertically.
        if self.cursor_row.saturating_add(style.size as u16) > self.rows {
            return Ok(());
        }
        let capabilities = TerminalEmulator::capabilities();
        let content = style.apply(content, &capabilities);
        self.writer.queue(style::PrintStyledContent(content))?;
        self.current_row_height = self.current_row_height.max(style.size as u16);
        Ok(())
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        if self.osc11_background {
            match (self.last_cleared_background_color, self.background_color) {
                (_, Some(Color::Rgb { r, g, b })) => {
                    // Set background via OSC 11 if we have an RGB color
                    write!(self.writer, "\x1b]11;#{r:02x}{g:02x}{b:02x}\x1b\\")?;
                }
                // If it was RGB and it no longer is, or we have no background now, clear it.
                (Some(Color::Rgb { .. }), Some(_)) | (_, None) => write!(self.writer, "\x1b]111\x1b\\")?,
                _ => (),
            };
        }
        self.last_cleared_background_color = self.background_color;
        let image_printer = self.image_printer.clone();
        if let ImagePrinter::Kitty(printer) = image_printer.as_ref() {
            printer.begin_redraw(&mut self.writer)?;
            // ED 2 also destroys Kitty image data. ED 0 from the origin clears
            // all text while leaving uploaded animations available for reuse.
            self.writer.queue(cursor::MoveTo(0, 0))?;
            self.writer.queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        } else {
            self.writer.queue(terminal::Clear(terminal::ClearType::All))?;
        }
        self.cursor_row = 0;
        self.current_row_height = 1;
        Ok(())
    }

    fn set_colors(&mut self, colors: Colors) -> io::Result<()> {
        // Save this for when the screen is cleared..
        self.background_color = colors.background;

        let colors = colors.into();
        self.writer.queue(style::ResetColor)?;
        self.writer.queue(style::SetColors(colors))?;
        Ok(())
    }

    fn set_background_color(&mut self, color: Color) -> io::Result<()> {
        self.background_color = Some(color);

        let color = color.into();
        self.writer.queue(style::SetBackgroundColor(color))?;
        Ok(())
    }

    fn set_cursor_boundaries(&mut self, rows: u16) {
        self.rows = rows;
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    fn print_image(&mut self, image: &Image, options: &PrintOptions) -> Result<(), PrintImageError> {
        let image_printer = self.image_printer.clone();
        image_printer.print(image.image(), options, self)?;
        self.cursor_row += options.rows;
        Ok(())
    }

    pub(crate) fn set_terminal_progress(&mut self, percentage: u8) {
        if !self.terminal_progress_enabled {
            return;
        }
        self.terminal_progress = percentage;
        let _ = write!(self.writer, "\x1b]9;4;1;{percentage}\x1b\\");
        let _ = self.writer.flush();
    }

    pub(crate) fn clear_terminal_progress(&mut self) {
        if !self.terminal_progress_enabled {
            return;
        }
        self.terminal_progress = 0;
        let _ = write!(self.writer, "\x1b]9;4;0\x1b\\");
        let _ = self.writer.flush();
    }

    pub(crate) fn suspend(&mut self) {
        self.clear_terminal_progress();
        if let ImagePrinter::Kitty(printer) = self.image_printer.as_ref() {
            printer.reset();
        }
        self.writer.deinit();
    }

    pub(crate) fn resume(&mut self) {
        let _ = self.writer.init();
        if self.terminal_progress_enabled && self.terminal_progress > 0 {
            let _ = write!(self.writer, "\x1b]9;4;1;{}\x1b\\", self.terminal_progress);
            let _ = self.writer.flush();
        }
    }
}

impl<I: TerminalWrite> TerminalIo for Terminal<I> {
    fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
        use TerminalCommand::*;
        match command {
            BeginUpdate => self.begin_update()?,
            EndUpdate => self.end_update()?,
            MoveTo { column, row } => self.move_to(*column, *row)?,
            MoveToRow(row) => self.move_to_row(*row)?,
            MoveToColumn(column) => self.move_to_column(*column)?,
            MoveDown(amount) => self.move_down(*amount)?,
            MoveRight(amount) => self.move_right(*amount)?,
            MoveLeft(amount) => self.move_left(*amount)?,
            MoveToNextLine => self.move_to_next_line()?,
            PrintText { content, style } => self.print_text(content, style)?,
            ClearScreen => self.clear_screen()?,
            SetColors(colors) => self.set_colors(*colors)?,
            SetBackgroundColor(color) => self.set_background_color(*color)?,
            SetCursorBoundaries { rows } => self.set_cursor_boundaries(*rows),
            Flush => self.flush()?,
            PrintImage { image, options } => self.print_image(image, options)?,
        };
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.cursor_row
    }
}

impl<I: TerminalWrite> Drop for Terminal<I> {
    fn drop(&mut self) {
        self.clear_terminal_progress();
        if self.osc11_background {
            if let Some(Color::Rgb { .. }) = self.background_color {
                let _ = write!(self.writer, "\x1b]111\x1b\\");
            }
        }
        if let ImagePrinter::Kitty(printer) = self.image_printer.as_ref() {
            printer.reset();
        }
        self.writer.deinit();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::image::{
        ImageSource,
        printer::ImageSpec,
        protocols::kitty::{KittyMode, KittyPrinter},
    };
    use image::{Delay, Frame, RgbaImage, codecs::gif::GifEncoder};
    use rstest::rstest;

    #[derive(Default)]
    struct Buffer(Vec<u8>);

    impl io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl TerminalWrite for Buffer {
        fn init(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn deinit(&mut self) {}
    }

    fn animation(printer: &ImagePrinter) -> Image {
        let file = tempfile::Builder::new().suffix(".gif").tempfile().unwrap();
        let mut encoder = GifEncoder::new(file.reopen().unwrap());
        for color in [image::Rgba([255, 0, 0, 255]), image::Rgba([0, 255, 0, 255])] {
            let buffer = RgbaImage::from_pixel(2, 2, color);
            encoder.encode_frame(Frame::from_parts(buffer, 0, 0, Delay::from_numer_denom_ms(1000, 1))).unwrap();
        }
        drop(encoder);
        let image = printer.register(ImageSpec::Filesystem(file.path().to_path_buf())).unwrap();
        Image::new(image, ImageSource::Generated)
    }

    fn options() -> PrintOptions {
        PrintOptions { columns: 4, rows: 2, z_index: 0, background_color: None, column_width: 8, row_height: 16 }
    }

    fn draw(terminal: &mut Terminal<Buffer>, images: &[(&Image, PrintOptions)]) -> String {
        terminal.writer.0.clear();
        terminal.execute(&TerminalCommand::BeginUpdate).unwrap();
        terminal.execute(&TerminalCommand::ClearScreen).unwrap();
        for (image, options) in images {
            terminal.execute(&TerminalCommand::MoveTo { column: 2, row: 3 }).unwrap();
            terminal
                .execute(&TerminalCommand::PrintImage { image: (*image).clone(), options: options.clone() })
                .unwrap();
        }
        terminal.execute(&TerminalCommand::EndUpdate).unwrap();
        String::from_utf8(terminal.writer.0.clone()).unwrap()
    }

    #[rstest]
    fn kitty_animation_survives_redraw(
        #[values(KittyMode::Local, KittyMode::Remote)] mode: KittyMode,
        #[values(false, true)] tmux: bool,
    ) {
        TerminalEmulator::disable_capability_detection();
        let printer = Arc::new(ImagePrinter::Kitty(KittyPrinter::new(mode, tmux).unwrap()));
        let image = animation(&printer);
        let mut terminal = Terminal::new(Buffer::default(), printer, false).unwrap();
        let first = draw(&mut terminal, &[(&image, options())]);
        assert!(first.contains("a=T"));
        assert!(first.contains("a=f"));
        assert!(first.contains("a=a"));

        let mut resized = options();
        resized.columns = 8;
        for opts in [options(), resized] {
            let next = draw(&mut terminal, &[(&image, opts)]);
            assert!(!next.contains("a=T"), "redraw must not retransmit the animation");
            assert!(!next.contains("a=f"));
            assert!(!next.contains("a=a"), "redraw must not alter animation playback");
            assert!(!next.contains("\x1b[2J"), "full-screen erase deletes Kitty image data");
            assert!(next.contains("a=p"));
        }

        // Hiding the image (including going backwards past its pause) releases it.
        let hidden = draw(&mut terminal, &[]);
        assert!(hidden.contains("d=I"));
        let shown_again = draw(&mut terminal, &[(&image, options())]);
        assert!(shown_again.contains("a=T"));

        // The alternate screen is cleared when resuming after suspension.
        terminal.suspend();
        terminal.resume();
        let resumed = draw(&mut terminal, &[(&image, options())]);
        assert!(resumed.contains("a=T"));
    }

    #[test]
    fn kitty_image_lifecycle() {
        TerminalEmulator::disable_capability_detection();
        let printer = Arc::new(ImagePrinter::Kitty(KittyPrinter::new(KittyMode::Remote, false).unwrap()));
        let create_image = || {
            let pixels = image::DynamicImage::new_rgba8(2, 2);
            Image::new(printer.register(ImageSpec::Generated(pixels)).unwrap(), ImageSource::Generated)
        };
        let first = create_image();
        let second = create_image();
        let mut terminal = Terminal::new(Buffer::default(), printer.clone(), false).unwrap();
        let initial = draw(&mut terminal, &[(&first, options()), (&first, options()), (&second, options())]);
        assert_eq!(initial.matches("a=T").count(), 2);
        assert_eq!(initial.matches("a=p").count(), 1);

        let next = draw(&mut terminal, &[(&first, options())]);
        assert_eq!(next.matches("d=I").count(), 1);
        assert!(next.contains("a=p"));
        assert!(!next.contains("a=T"));

        // A reloaded/generated image must not reuse another image's terminal data.
        let replacement = create_image();
        let reloaded = draw(&mut terminal, &[(&replacement, options())]);
        assert!(reloaded.contains("a=T"));
        assert_eq!(reloaded.matches("d=I").count(), 1);

        // Cleanup must work even when text rendering would be clipped below the screen.
        terminal.execute(&TerminalCommand::ClearScreen).unwrap();
        terminal.execute(&TerminalCommand::SetCursorBoundaries { rows: 10 }).unwrap();
        terminal.execute(&TerminalCommand::MoveToRow(10)).unwrap();
        terminal.writer.0.clear();
        terminal.execute(&TerminalCommand::EndUpdate).unwrap();
        assert!(String::from_utf8(terminal.writer.0.clone()).unwrap().contains("d=I"));
    }

    #[test]
    fn other_protocols_keep_full_screen_clear() {
        TerminalEmulator::disable_capability_detection();
        let mut terminal = Terminal::new(Buffer::default(), Arc::new(ImagePrinter::Null), false).unwrap();
        let output = draw(&mut terminal, &[]);
        assert!(output.contains("\x1b[2J"));
    }
}
