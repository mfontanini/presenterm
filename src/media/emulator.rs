use super::query::TerminalCapabilities;
use crate::{GraphicsMode, media::kitty::KittyMode};
use std::env;
use strum::IntoEnumIterator;

#[derive(Debug, strum::EnumIter)]
pub enum TerminalEmulator {
    Kitty,
    Iterm2,
    WezTerm,
    Mintty,
    Konsole,
    Foot,
    Yaft,
    Mlterm,
    St,
    Xterm,
    Ghostty,
    Unknown,
}

impl TerminalEmulator {
    pub fn detect() -> Self {
        let term = env::var("TERM").unwrap_or_default();
        let term_program = env::var("TERM_PROGRAM").unwrap_or_default();
        for emulator in Self::iter() {
            if emulator.is_detected(&term, &term_program) {
                return emulator;
            }
        }
        TerminalEmulator::Unknown
    }

    pub fn preferred_protocol(&self) -> GraphicsMode {
        let capabilities = TerminalCapabilities::query().unwrap_or_default();
        let modes = [
            GraphicsMode::Iterm2,
            GraphicsMode::Kitty { mode: KittyMode::Local, inside_tmux: capabilities.tmux },
            GraphicsMode::Kitty { mode: KittyMode::Remote, inside_tmux: capabilities.tmux },
            #[cfg(feature = "sixel")]
            GraphicsMode::Sixel,
            GraphicsMode::AsciiBlocks,
        ];
        for mode in modes {
            if self.supports_graphics_mode(&mode, &capabilities) {
                return mode;
            }
        }
        unreachable!("ascii blocks is always supported")
    }

    fn is_detected(&self, term: &str, term_program: &str) -> bool {
        match self {
            TerminalEmulator::Kitty => term.contains("kitty"),
            TerminalEmulator::Iterm2 => term_program.contains("iTerm"),
            TerminalEmulator::WezTerm => term_program.contains("WezTerm"),
            TerminalEmulator::Mintty => term_program.contains("mintty"),
            TerminalEmulator::Konsole => env::var("KONSOLE_VERSION").is_ok(),
            TerminalEmulator::Foot => ["foot", "foot-extra"].contains(&term),
            TerminalEmulator::Yaft => term == "yaft-256color",
            TerminalEmulator::Mlterm => term == "mlterm",
            TerminalEmulator::St => term == "st-256color",
            TerminalEmulator::Xterm => ["xterm", "xterm-256color"].contains(&term),
            TerminalEmulator::Ghostty => term_program.contains("ghostty"),
            TerminalEmulator::Unknown => true,
        }
    }

    fn supports_graphics_mode(&self, mode: &GraphicsMode, capabilities: &TerminalCapabilities) -> bool {
        match (mode, self) {
            (GraphicsMode::Kitty { mode, .. }, Self::Kitty | Self::WezTerm | Self::Ghostty) => match mode {
                KittyMode::Local => capabilities.kitty_local,
                KittyMode::Remote => true,
            },
            (GraphicsMode::Kitty { mode: KittyMode::Local, .. }, Self::Unknown) => {
                // If we don't know the emulator but we detected that we support kitty use it,
                // **unless** we are inside tmux and we "guess" that we're using wezterm. This is
                // because wezterm's support for unicode placeholders (needed to display images in
                // kitty when inside tmux) is not implemented (see
                // https://github.com/wez/wezterm/issues/986).
                //
                // We can only really guess it's wezterm by checking environment variables and will
                // not work if you started tmux on a different emulator and are running presenterm
                // in wezterm.
                capabilities.kitty_local && (!capabilities.tmux || !Self::guess_wezterm())
            }
            (GraphicsMode::Kitty { mode: KittyMode::Remote, .. }, Self::Unknown) => {
                // Same as the above
                capabilities.kitty_remote && (!capabilities.tmux || !Self::guess_wezterm())
            }
            (GraphicsMode::Iterm2, Self::Iterm2 | Self::WezTerm | Self::Mintty | Self::Konsole) => true,
            (GraphicsMode::AsciiBlocks, _) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, Self::Foot | Self::Yaft | Self::Mlterm) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, Self::St | Self::Xterm | Self::Unknown) => capabilities.sixel,
            _ => false,
        }
    }

    fn guess_wezterm() -> bool {
        env::var("WEZTERM_EXECUTABLE").is_ok()
    }
}
