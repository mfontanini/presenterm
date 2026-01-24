use super::{GraphicsMode, capabilities::TerminalCapabilities, image::protocols::kitty::KittyMode};
use std::{env, sync::OnceLock};
use strum::IntoEnumIterator;

static CAPABILITIES: OnceLock<TerminalCapabilities> = OnceLock::new();

#[derive(Debug, strum::EnumIter)]
pub enum TerminalEmulator {
    Iterm2,
    WezTerm,
    Ghostty,
    Mintty,
    Kitty,
    Konsole,
    Foot,
    Yaft,
    Mlterm,
    St,
    Xterm,
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

    pub(crate) fn capabilities() -> TerminalCapabilities {
        CAPABILITIES.get_or_init(|| TerminalCapabilities::query().unwrap_or_default()).clone()
    }

    pub(crate) fn disable_capability_detection() {
        CAPABILITIES.get_or_init(TerminalCapabilities::default);
    }

    pub fn preferred_protocol(&self) -> GraphicsMode {
        let capabilities = Self::capabilities();
        // Note: the order here is very important. In particular:
        //
        // * We prioritize checking for iterm2 support as the default for terminals that support
        // it.
        // * Kitty local is checked before remote since remote should also work when local is
        // supported but local is more efficient.
        // * Sixel is not great so we use it as a last resort.
        // * ASCII blocks is supported by all terminals so it must come last.
        let modes = [
            GraphicsMode::Iterm2,
            GraphicsMode::Iterm2Multipart,
            GraphicsMode::Kitty { mode: KittyMode::Local },
            GraphicsMode::Kitty { mode: KittyMode::Remote },
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
            TerminalEmulator::Iterm2 => {
                term_program.contains("iTerm") || env::var("LC_TERMINAL").is_ok_and(|c| c.contains("iTerm"))
            }
            TerminalEmulator::WezTerm => term_program.contains("WezTerm") || env::var("WEZTERM_EXECUTABLE").is_ok(),
            TerminalEmulator::Mintty => term_program.contains("mintty"),
            TerminalEmulator::Ghostty => term_program.contains("ghostty"),
            TerminalEmulator::Kitty => term.contains("kitty"),
            TerminalEmulator::Konsole => env::var("KONSOLE_VERSION").is_ok(),
            TerminalEmulator::Foot => ["foot", "foot-extra"].contains(&term),
            TerminalEmulator::Yaft => term == "yaft-256color",
            TerminalEmulator::Mlterm => term == "mlterm",
            TerminalEmulator::St => term == "st-256color",
            TerminalEmulator::Xterm => ["xterm", "xterm-256color"].contains(&term),
            TerminalEmulator::Unknown => true,
        }
    }

    fn supports_graphics_mode(&self, mode: &GraphicsMode, capabilities: &TerminalCapabilities) -> bool {
        match (mode, self) {
            // Use the kitty protocol in any terminal that supports the kitty graphics protocol.
            //
            // Note that this could potentially break for terminals that don't support the unicode
            // placeholder part of the spec which is required for this to work under tmux, but it's
            // not our fault terminals half implement the protocol.
            (GraphicsMode::Kitty { mode, .. }, _) => match mode {
                KittyMode::Local => capabilities.kitty_local,
                KittyMode::Remote => capabilities.kitty_remote,
            },
            // All of these support the iterm2 protocol
            (GraphicsMode::Iterm2, Self::Iterm2 | Self::WezTerm | Self::Mintty | Self::Konsole) => true,
            // Only iterm2 supports the iterm2 protocol in multipart form.
            (GraphicsMode::Iterm2Multipart, Self::Iterm2) => true,
            // All terminals support ascii protocol
            (GraphicsMode::AsciiBlocks, _) => true,
            (GraphicsMode::Sixel, Self::Foot | Self::Yaft | Self::Mlterm) => true,
            (GraphicsMode::Sixel, Self::St | Self::Xterm | Self::Unknown) => capabilities.sixel,
            _ => false,
        }
    }
}
