use super::kitty::local_mode_supported;
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
    Unknown,
}

impl TerminalEmulator {
    pub fn is_inside_tmux() -> bool {
        env::var("TERM_PROGRAM").ok().as_deref() == Some("tmux")
    }

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
            TerminalEmulator::Unknown => true,
        }
    }

    fn supports_graphics_mode(&self, mode: &GraphicsMode) -> bool {
        match (mode, self) {
            (GraphicsMode::Kitty { mode, inside_tmux }, Self::Kitty | Self::WezTerm) => match mode {
                KittyMode::Local => local_mode_supported(*inside_tmux).unwrap_or_default(),
                KittyMode::Remote => true,
            },
            (GraphicsMode::Iterm2, Self::Iterm2 | Self::WezTerm | Self::Mintty | Self::Konsole) => true,
            (GraphicsMode::AsciiBlocks, _) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, Self::Foot | Self::Yaft | Self::Mlterm) => true,
            #[cfg(feature = "sixel")]
            (GraphicsMode::Sixel, Self::St | Self::Xterm) => supports_sixel().unwrap_or_default(),
            _ => false,
        }
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
