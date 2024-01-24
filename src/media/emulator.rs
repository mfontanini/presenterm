use super::kitty::local_mode_supported;
use crate::{GraphicsMode, KittyMode};
use std::env;

#[derive(Debug)]
pub enum TerminalEmulator {
    Kitty,
    Iterm2,
    WezTerm,
    Mintty,
    Unknown,
}

impl TerminalEmulator {
    pub fn is_inside_tmux() -> bool {
        env::var("TERM_PROGRAM").ok().as_deref() == Some("tmux")
    }

    pub fn detect() -> Self {
        if Self::is_kitty() {
            Self::Kitty
        } else if Self::is_iterm2() {
            Self::Iterm2
        } else if Self::is_wezterm() {
            Self::WezTerm
        } else if Self::is_mintty() {
            Self::Mintty
        } else {
            Self::Unknown
        }
    }

    pub fn preferred_protocol(&self) -> GraphicsMode {
        let inside_tmux = Self::is_inside_tmux();
        let modes = [
            GraphicsMode::Iterm2,
            GraphicsMode::Kitty { mode: KittyMode::Local, inside_tmux },
            GraphicsMode::Kitty { mode: KittyMode::Remote, inside_tmux },
            #[cfg(feature = "sixel")]
            GraphicsMode::Sixel,
            GraphicsMode::AsciiBlocks,
        ];
        for mode in modes {
            if self.supports_graphics_mode(&mode) {
                return mode;
            }
        }
        unreachable!("ascii blocks is always supported")
    }

    fn supports_graphics_mode(&self, mode: &GraphicsMode) -> bool {
        match (mode, self) {
            (GraphicsMode::Kitty { mode, inside_tmux }, Self::Kitty | Self::WezTerm) => match mode {
                KittyMode::Local => local_mode_supported(*inside_tmux).unwrap_or_default(),
                KittyMode::Remote => true,
            },
            (GraphicsMode::Iterm2, Self::Iterm2 | Self::WezTerm | Self::Mintty) => true,
            (GraphicsMode::AsciiBlocks, _) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, _) => viuer::is_sixel_supported(),
            _ => false,
        }
    }

    fn is_kitty() -> bool {
        let Ok(term) = env::var("TERM") else {
            return false;
        };
        term.contains("kitty")
    }

    fn is_iterm2() -> bool {
        Self::load_term_program().map(|term| term.contains("iTerm")).unwrap_or_default()
    }

    fn is_wezterm() -> bool {
        Self::load_term_program().as_deref() == Some("WezTerm")
    }

    fn is_mintty() -> bool {
        Self::load_term_program().map(|term| term.contains("mintty")).unwrap_or_default()
    }

    fn load_term_program() -> Option<String> {
        env::var("TERM_PROGRAM").ok()
    }
}
