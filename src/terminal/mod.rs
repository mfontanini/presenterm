pub(crate) mod ansi;
pub(crate) mod capabilities;
pub(crate) mod emulator;
pub(crate) mod image;
pub(crate) mod printer;
pub(crate) mod virt;

pub(crate) use printer::{Terminal, TerminalWrite, should_hide_cursor};

/// Check whether stdout is a terminal with non-zero dimensions.
pub(crate) fn has_sized_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal() && crossterm::terminal::size().is_ok_and(|(cols, rows)| cols > 0 && rows > 0)
}

#[derive(Clone, Debug)]
pub enum GraphicsMode {
    Iterm2,
    Iterm2Multipart,
    Kitty { mode: image::protocols::kitty::KittyMode },
    AsciiBlocks,
    Raw,
    Sixel,
}
