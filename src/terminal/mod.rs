pub(crate) mod emulator;
pub(crate) mod image;
pub(crate) mod query;

#[derive(Clone, Debug)]
pub enum GraphicsMode {
    Iterm2,
    Kitty {
        mode: image::protocols::kitty::KittyMode,
        inside_tmux: bool,
    },
    AsciiBlocks,
    #[cfg(feature = "sixel")]
    Sixel,
}
