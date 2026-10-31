use super::image::protocols::kitty::{Action, ControlCommand, ControlOption, ImageFormat, TransmissionMedium};
use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::{
    QueueableCommand,
    cursor::{self},
    style::Print,
    terminal,
};
use image::{DynamicImage, EncodableLayout};
use std::{
    env,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tempfile::NamedTempFile;

#[derive(Default, Debug, Clone)]
pub(crate) struct TerminalCapabilities {
    pub(crate) kitty_local: bool,
    pub(crate) kitty_remote: bool,
    pub(crate) sixel: bool,
    pub(crate) tmux: bool,
    pub(crate) font_size: bool,
    pub(crate) fractional_font_size: bool,
    pub(crate) hyperlinks: bool,
}

impl TerminalCapabilities {
    pub(crate) fn is_inside_tmux() -> bool {
        env::var("TERM_PROGRAM").ok().as_deref() == Some("tmux")
    }

    pub(crate) fn query() -> io::Result<Self> {
        let tmux = Self::is_inside_tmux();
        let mut file = NamedTempFile::new()?;
        let image = DynamicImage::new_rgba8(1, 1).into_rgba8();
        let image_bytes = image.as_raw().as_bytes();
        file.write_all(image_bytes)?;
        file.flush()?;
        let Some(path) = file.path().as_os_str().to_str() else {
            return Ok(Default::default());
        };
        let encoded_path = STANDARD.encode(path);

        let base_image_id = fastrand::u32(0..=u32::MAX);
        let ids = KittyImageIds { local: base_image_id, remote: base_image_id.wrapping_add(1) };
        Self::write_kitty_local_query(ids.local, encoded_path, tmux)?;
        Self::write_kitty_remote_query(ids.remote, image_bytes, tmux)?;
        let (start, sequence, end) = match tmux {
            true => ("\x1bPtmux;", "\x1b\x1b", "\x1b\\"),
            false => ("", "\x1b", ""),
        };
        let _guard = RawModeGuard::new()?;
        let mut stdout = io::stdout();
        write!(stdout, "{start}{sequence}[c{end}")?;
        stdout.flush()?;

        // Spawn a thread to "save us" in case we don't get an answer from the terminal.
        let running = Arc::new(AtomicBool::new(true));
        Self::launch_timeout_trigger(running.clone());

        let response = Self::build_capabilities(ids);
        running.store(false, Ordering::Relaxed);

        let mut response = response?;
        response.tmux = tmux;
        Ok(response)
    }

    /// Detect OSC 8 hyperlink support from the environment.
    pub(crate) fn hyperlinks_supported_from_env() -> bool {
        Self::hyperlinks_supported(|name| env::var(name).ok())
    }

    /// The hyperlink support override requested via `FORCE_HYPERLINK`, if any.
    pub(crate) fn hyperlink_force_override() -> Option<bool> {
        Self::parse_force_hyperlink(env::var("FORCE_HYPERLINK").ok())
    }

    fn parse_force_hyperlink(value: Option<String>) -> Option<bool> {
        let value = value?;
        Some(!matches!(value.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "no" | "off"))
    }

    /// Whether the terminal emulator supports OSC 8 hyperlinks.
    ///
    /// There is no escape sequence to query hyperlink support so, like the rest of the ecosystem,
    /// this relies on environment variable heuristics. The heuristic errs on the side of "no": a
    /// false negative merely causes URLs to be displayed inline next to their label, whereas a
    /// false positive would cause them to be silently dropped by the terminal.
    fn hyperlinks_supported<F: Fn(&str) -> Option<String>>(var: F) -> bool {
        // Allow forcing hyperlinks on/off, following the convention used by other tools.
        if let Some(force) = Self::parse_force_hyperlink(var("FORCE_HYPERLINK")) {
            return force;
        }
        // Multiplexers swallow OSC 8 sequences: GNU screen entirely, and tmux only passes them
        // through on >= 3.4 when the outer terminal supports them, which we can't detect from in
        // here. Note that these must be checked first since environment variables set by the
        // terminal that hosts the multiplexer (e.g. `VTE_VERSION`) leak into its sessions, and
        // `TERM` must be checked too since only it survives into ssh sessions.
        if var("TMUX").is_some() || var("TERM_PROGRAM").as_deref() == Some("tmux") {
            return false;
        }
        let term = var("TERM").unwrap_or_default();
        let term_is = |name: &str, prefix: &str| term == name || term.starts_with(prefix);
        if term_is("screen", "screen-") || term.starts_with("screen.") || term_is("tmux", "tmux-") {
            return false;
        }
        // Terminals that advertise themselves via $TERM.
        if ["xterm-kitty", "alacritty", "foot", "foot-extra", "xterm-ghostty", "contour", "rio"]
            .contains(&term.as_str())
        {
            return true;
        }
        // Terminals that advertise themselves via $TERM_PROGRAM, falling back to $LC_TERMINAL
        // which iTerm2 propagates over ssh.
        let program = var("TERM_PROGRAM").or_else(|| var("LC_TERMINAL")).unwrap_or_default();
        if ["iTerm.app", "iTerm2", "WezTerm", "ghostty", "vscode", "Hyper", "mintty", "terminology", "rio", "Tabby"]
            .contains(&program.as_str())
        {
            return true;
        }
        // VTE based terminals (GNOME terminal, xfce4-terminal, tilix, etc) support these since 0.50.
        if var("VTE_VERSION").and_then(|version| version.parse::<u32>().ok()).is_some_and(|version| version >= 5000) {
            return true;
        }
        // Konsole, Windows Terminal, DomTerm, and jetbrains IDE terminals.
        var("KONSOLE_VERSION").is_some()
            || var("WT_SESSION").is_some()
            || var("DOMTERM").is_some()
            || var("TERMINAL_EMULATOR").as_deref() == Some("JetBrains-JediTerm")
    }

    fn build_capabilities(ids: KittyImageIds) -> io::Result<TerminalCapabilities> {
        let mut response = Self::parse_response(io::stdin(), ids)?;

        // Use kitty's font size protocol to write 1 character using size 2. If after writing the
        // cursor has moves 2 columns, the protocol is supported.
        let mut stdout = io::stdout();
        stdout.queue(terminal::EnterAlternateScreen)?;
        stdout.queue(cursor::MoveTo(0, 0))?;
        stdout.queue(Print("\x1b]66;s=2; \x1b\\"))?;
        stdout.queue(Print("\x1b]66;n=1:d=2; \x1b\\"))?;
        stdout.flush()?;
        let position = cursor::position()?.0;
        if position == 1 {
            // If we only moved one, then only the fractional worked.
            response.fractional_font_size = true;
        } else if position == 2 {
            // If we only moved 2 then the scaled font size one worked.
            response.font_size = true;
        } else if position == 3 {
            // 3 -> both worked.
            response.font_size = true;
            response.fractional_font_size = true;
        }
        stdout.queue(terminal::LeaveAlternateScreen)?;
        stdout.flush()?;
        Ok(response)
    }

    fn write_kitty_local_query(image_id: u32, path: String, tmux: bool) -> io::Result<()> {
        let options = &[
            ControlOption::Format(ImageFormat::Rgba),
            ControlOption::Action(Action::Query),
            ControlOption::Medium(TransmissionMedium::LocalFile),
            ControlOption::ImageId(image_id),
            ControlOption::Width(1),
            ControlOption::Height(1),
        ];
        let command = ControlCommand { options, payload: path, tmux };
        write!(io::stdout(), "{command}")
    }

    fn write_kitty_remote_query(image_id: u32, image: &[u8], tmux: bool) -> io::Result<()> {
        let payload = STANDARD.encode(image);
        let options = &[
            ControlOption::Format(ImageFormat::Rgba),
            ControlOption::Action(Action::Query),
            ControlOption::Medium(TransmissionMedium::Direct),
            ControlOption::ImageId(image_id),
            ControlOption::Width(1),
            ControlOption::Height(1),
        ];
        // The image is small enough to fit in a single request so we don't need to bother with
        // chunks here.
        let command = ControlCommand { options, payload, tmux };
        write!(io::stdout(), "{command}")
    }

    fn parse_response<T: io::Read>(mut term: T, ids: KittyImageIds) -> io::Result<Self> {
        let mut buffer = [0_u8; 128];
        let mut state = QueryParseState::default();
        let mut capabilities = TerminalCapabilities::default();
        loop {
            let bytes_read = term.read(&mut buffer)?;
            if bytes_read == 0 {
                return Ok(capabilities);
            }
            for next in &buffer[0..bytes_read] {
                let next = char::from(*next);
                let Some(output) = state.update(next) else {
                    continue;
                };
                match output {
                    Response::KittySupported { image_id } => {
                        if image_id == ids.local {
                            capabilities.kitty_local = true;
                        } else if image_id == ids.remote {
                            capabilities.kitty_remote = true;
                        }
                    }
                    Response::Capabilities { sixel } => {
                        capabilities.sixel = sixel;
                        return Ok(capabilities);
                    }
                    Response::StatusReport => {
                        return Ok(capabilities);
                    }
                }
            }
        }
    }

    fn launch_timeout_trigger(running: Arc<AtomicBool>) {
        // Spawn a thread that will wait a second and if we still are running, will request the
        // device status report straight from whoever is on top of us (tmux or terminal if no
        // tmux), which will cause it to answer and wake up our main thread that's reading on
        // stdin.
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            if !running.load(Ordering::Relaxed) {
                return;
            }
            let _ = write!(io::stdout(), "\x1b[5n");
            let _ = io::stdout().flush();
        });
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

#[derive(Default)]
struct QueryParseState {
    data: String,
    current: ResponseType,
}

impl QueryParseState {
    fn update(&mut self, next: char) -> Option<Response> {
        match &self.current {
            ResponseType::Unknown => {
                match (self.data.as_str(), next) {
                    (_, '\x1b') => {
                        *self = Default::default();
                        return None;
                    }
                    ("[", '?') => {
                        self.current = ResponseType::Capabilities;
                    }
                    ("[", '0') => {
                        self.current = ResponseType::StatusReport;
                    }
                    ("_Gi", '=') => {
                        self.current = ResponseType::Kitty;
                    }
                    _ => (),
                };
                self.data.push(next);
            }
            ResponseType::Kitty => match next {
                '\\' => {
                    let response = self.build_kitty_response();
                    *self = Default::default();
                    return response;
                }
                _ => {
                    self.data.push(next);
                }
            },
            ResponseType::Capabilities => match next {
                'c' => {
                    let mut caps = self.data[2..].split(';');
                    let sixel = caps.any(|cap| cap == "4");
                    *self = Default::default();
                    return Some(Response::Capabilities { sixel });
                }
                _ => self.data.push(next),
            },
            ResponseType::StatusReport => match next {
                'n' => {
                    *self = Default::default();
                    return Some(Response::StatusReport);
                }
                _ => self.data.push(next),
            },
        };
        None
    }

    fn build_kitty_response(&self) -> Option<Response> {
        if !self.data.ends_with(";OK\x1b") {
            return None;
        }
        let (_, rest) = self.data.split_once("_Gi=").expect("no kitty prefix");
        let (image_id, _) = rest.split_once(';')?;
        let image_id = image_id.parse::<u32>().ok()?;
        Some(Response::KittySupported { image_id })
    }
}

#[derive(Default)]
enum ResponseType {
    #[default]
    Unknown,
    Kitty,
    Capabilities,
    StatusReport,
}

enum Response {
    KittySupported { image_id: u32 },
    Capabilities { sixel: bool },
    StatusReport,
}

struct KittyImageIds {
    local: u32,
    remote: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;
    use rstest::rstest;

    #[rstest]
    #[case::kitty_local("\x1b_Gi=42;OK\x1b\\\x1b[?c", true, false, false)]
    #[case::kitty_remote("\x1b_Gi=43;OK\x1b\\\x1b[?c", false, true, false)]
    #[case::kitty_both("\x1b_Gi=42;OK\x1b\\\x1b_Gi=43;OK\x1b\\\x1b[?c", true, true, false)]
    #[case::kitty_flipped("\x1b_Gi=43;OK\x1b\\\x1b_Gi=42;OK\x1b\\\x1b[?c", true, true, false)]
    #[case::all("\x1b_Gi=42;OK\x1b\\\x1b_Gi=43;OK\x1b\\\x1b[?4c", true, true, true)]
    #[case::none("\x1b[?c", false, false, false)]
    #[case::sixel_single("\x1b[?4c", false, false, true)]
    #[case::sixel_first("\x1b[?4;42c", false, false, true)]
    #[case::sixel_middle("\x1b[?1337;4;42c", false, false, true)]
    fn detection(#[case] input: &str, #[case] kitty_local: bool, #[case] kitty_remote: bool, #[case] sixel: bool) {
        let input = Cursor::new(input);
        let ids = KittyImageIds { local: 42, remote: 43 };
        let capabilities = TerminalCapabilities::parse_response(input, ids).expect("reading failed");
        assert_eq!(capabilities.kitty_local, kitty_local);
        assert_eq!(capabilities.kitty_remote, kitty_remote);
        assert_eq!(capabilities.sixel, sixel);
    }

    #[rstest]
    #[case::nothing(&[], false)]
    #[case::dumb_xterm(&[("TERM", "xterm-256color")], false)]
    #[case::apple_terminal(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "Apple_Terminal")], false)]
    #[case::iterm(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "iTerm.app")], true)]
    #[case::iterm_over_ssh(&[("TERM", "xterm-256color"), ("LC_TERMINAL", "iTerm2")], true)]
    #[case::kitty(&[("TERM", "xterm-kitty")], true)]
    #[case::alacritty(&[("TERM", "alacritty")], true)]
    #[case::wezterm(&[("TERM_PROGRAM", "WezTerm")], true)]
    #[case::ghostty(&[("TERM", "xterm-ghostty"), ("TERM_PROGRAM", "ghostty")], true)]
    #[case::vte(&[("TERM", "xterm-256color"), ("VTE_VERSION", "7802")], true)]
    #[case::old_vte(&[("TERM", "xterm-256color"), ("VTE_VERSION", "4999")], false)]
    #[case::konsole(&[("KONSOLE_VERSION", "230800")], true)]
    #[case::windows_terminal(&[("WT_SESSION", "some-guid")], true)]
    #[case::jetbrains(&[("TERMINAL_EMULATOR", "JetBrains-JediTerm")], true)]
    #[case::tmux(&[("TMUX", "/tmp/tmux-1/default,42,0"), ("TERM_PROGRAM", "tmux"), ("VTE_VERSION", "7802")], false)]
    #[case::screen(&[("TERM", "screen-256color"), ("LC_TERMINAL", "iTerm2")], false)]
    #[case::ssh_from_tmux(&[("TERM", "tmux-256color"), ("LC_TERMINAL", "iTerm2")], false)]
    #[case::force_on(&[("FORCE_HYPERLINK", "1"), ("TERM", "xterm-256color")], true)]
    #[case::force_on_beats_tmux(&[("FORCE_HYPERLINK", "1"), ("TMUX", "/tmp/tmux-1/default,42,0")], true)]
    #[case::force_off(&[("FORCE_HYPERLINK", "0"), ("TERM_PROGRAM", "iTerm.app")], false)]
    #[case::force_off_false(&[("FORCE_HYPERLINK", "false"), ("TERM_PROGRAM", "iTerm.app")], false)]
    #[case::force_off_no(&[("FORCE_HYPERLINK", "No"), ("TERM_PROGRAM", "iTerm.app")], false)]
    #[case::force_off_off(&[("FORCE_HYPERLINK", "off"), ("TERM_PROGRAM", "iTerm.app")], false)]
    fn hyperlink_detection(#[case] vars: &[(&str, &str)], #[case] expected: bool) {
        let lookup = |name: &str| vars.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string());
        assert_eq!(TerminalCapabilities::hyperlinks_supported(lookup), expected);
    }
}
