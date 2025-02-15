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

    fn build_capabilities(ids: KittyImageIds) -> io::Result<TerminalCapabilities> {
        let mut response = Self::parse_response(io::stdin(), ids)?;

        // Use kitty's font size protocol to write 1 character using size 2. If after writing the
        // cursor has moves 2 columns, the protocol is supported.
        let mut stdout = io::stdout();
        stdout.queue(terminal::EnterAlternateScreen)?;
        stdout.queue(cursor::MoveTo(0, 0))?;
        stdout.queue(Print("\x1b]66;s=2; \x1b\\"))?;
        stdout.flush()?;
        let position = cursor::position()?;
        if position.0 == 2 {
            response.font_size = true;
        }
        stdout.queue(terminal::LeaveAlternateScreen)?;
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
}
