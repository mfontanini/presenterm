use super::kitty::KittyMode;

#[derive(Clone, Debug)]
pub enum GraphicsMode {
    Iterm2,
    Kitty {
        mode: KittyMode,
        inside_tmux: bool,
    },
    AsciiBlocks,
    #[cfg(feature = "sixel")]
    Sixel,
}
