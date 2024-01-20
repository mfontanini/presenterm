use super::kitty::KittyMode;
use viuer::{get_kitty_support, is_iterm_supported, KittySupport};

#[derive(Clone, Debug)]
pub enum GraphicsMode {
    Iterm2,
    Kitty(KittyMode),
    AsciiBlocks,
    #[cfg(feature = "sixel")]
    Sixel,
}

impl Default for GraphicsMode {
    fn default() -> Self {
        let modes = &[
            Self::Iterm2,
            Self::Kitty(KittyMode::Local),
            Self::Kitty(KittyMode::Remote),
            #[cfg(feature = "sixel")]
            Self::Sixel,
            Self::AsciiBlocks,
        ];
        for mode in modes {
            if mode.is_supported() {
                return mode.clone();
            }
        }
        Self::AsciiBlocks
    }
}

impl GraphicsMode {
    pub fn is_supported(&self) -> bool {
        match self {
            Self::Iterm2 => is_iterm_supported(),
            Self::Kitty(KittyMode::Local) => get_kitty_support() == KittySupport::Local,
            Self::Kitty(KittyMode::Remote) => get_kitty_support() == KittySupport::Remote,
            Self::AsciiBlocks => true,
            #[cfg(feature = "sixel")]
            Self::Sixel => viuer::is_sixel_supported(),
        }
    }

    pub fn detect_graphics_protocol() {
        viuer::is_iterm_supported();
        viuer::get_kitty_support();
        #[cfg(feature = "sixel")]
        viuer::is_sixel_supported();
    }
}
