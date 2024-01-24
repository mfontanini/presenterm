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

impl GraphicsMode {
    pub fn detect_graphics_protocol() {
        viuer::is_iterm_supported();
        viuer::get_kitty_support();
        #[cfg(feature = "sixel")]
        viuer::is_sixel_supported();
    }
}
