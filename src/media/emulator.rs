use super::kitty::local_mode_supported;
use crate::{media::kitty::KittyMode, GraphicsMode};
use std::env;

#[derive(Debug)]
pub enum TerminalEmulator {
    Kitty,
    Iterm2,
    WezTerm,
    Mintty,
    Foot,
    Yaft,
    Mlterm,
    Unknown,
    St,
    Xterm,
}

impl TerminalEmulator {
    pub fn is_inside_tmux() -> bool {
        env::var("TERM_PROGRAM").ok().as_deref() == Some("tmux")
    }

    pub fn detect() -> Self {
        let term = env::var("TERM").unwrap_or_default();
        if Self::is_kitty(&term) {
            Self::Kitty
        } else if Self::is_iterm2() {
            Self::Iterm2
        } else if Self::is_wezterm() {
            Self::WezTerm
        } else if Self::is_mintty() {
            Self::Mintty
        } else if Self::is_foot(&term) {
            Self::Foot
        } else if Self::is_mlterm(&term) {
            Self::Mlterm
        } else if Self::is_yaft(&term) {
            Self::Yaft
        } else if Self::is_st(&term) {
            Self::St
        } else if Self::is_xterm(&term) {
            Self::Xterm
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
            (GraphicsMode::Sixel, Self::Foot | Self::Yaft | Self::Mlterm) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, Self::St | Self::Xterm) => supports_sixel().unwrap_or_default(),
            _ => false,
        }
    }

    fn is_kitty(term: &str) -> bool {
        term.contains("kitty")
    }

    fn is_foot(term: &str) -> bool {
        term == "foot" || term == "foot-extra"
    }

    fn is_mlterm(term: &str) -> bool {
        term == "mlterm"
    }

    fn is_yaft(term: &str) -> bool {
        term == "yaft-256color"
    }

    fn is_st(term: &str) -> bool {
        term == "st-256color"
    }

    fn is_xterm(term: &str) -> bool {
        term == "xterm" || term == "xterm-256color"
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

#[cfg(feature = "sixel")]
fn supports_sixel() -> std::io::Result<bool> {
    use console::{Key, Term};
    use std::io::Write;

    let mut term = Term::stdout();

    write!(&mut term, "\x1b[c")?;
    term.flush()?;

    let mut response = String::new();
    while let Ok(key) = term.read_key() {
        if let Key::Char(c) = key {
            response.push(c);
            if c == 'c' {
                break;
            }
        }
    }
    Ok(response.contains(";4;") || response.contains(";4c"))
}
