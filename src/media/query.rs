use super::kitty::{Action, ControlCommand, ControlOption, ImageFormat, TransmissionMedium};
use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use image::{DynamicImage, EncodableLayout};
use std::{
    env,
    io::{self, Write},
};
use tempfile::NamedTempFile;

#[derive(Default, Debug)]
pub(crate) struct TerminalCapabilities {
    pub(crate) kitty_local: bool,
    pub(crate) sixel: bool,
    pub(crate) tmux: bool,
}

impl TerminalCapabilities {
    pub(crate) fn is_inside_tmux() -> bool {
        env::var("TERM_PROGRAM").ok().as_deref() == Some("tmux")
    }

    pub(crate) fn query() -> io::Result<Self> {
        let tmux = Self::is_inside_tmux();
        let mut file = NamedTempFile::new()?;
        let image = DynamicImage::new_rgba8(1, 1);
        file.write_all(image.into_rgba8().as_raw().as_bytes())?;
        file.flush()?;
        let Some(path) = file.path().as_os_str().to_str() else {
            return Ok(Default::default());
        };
        let encoded_path = STANDARD.encode(path);

        let options = &[
            ControlOption::Format(ImageFormat::Rgba),
            ControlOption::Action(Action::Query),
            ControlOption::Medium(TransmissionMedium::LocalFile),
            ControlOption::ImageId(rand::random()),
            ControlOption::Width(1),
            ControlOption::Height(1),
        ];
        let command = ControlCommand { options, payload: encoded_path, tmux };
        let (start, sequence, end) = match tmux {
            true => ("\x1bPtmux;", "\x1b\x1b", "\x1b\\"),
            false => ("", "\x1b", ""),
        };
        let _guard = RawModeGuard::new()?;
        let mut stdout = io::stdout();
        write!(stdout, "{command}{start}{sequence}[c{end}")?;
        stdout.flush()?;
        let mut response = Self::parse_response(io::stdin())?;
        response.tmux = tmux;
        Ok(response)
    }

    fn parse_response<T: io::Read>(mut term: T) -> io::Result<Self> {
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
                    Response::KittySupported => {
                        capabilities.kitty_local = true;
                    }
                    Response::Capabilities { sixel } => {
                        capabilities.sixel = sixel;
                        return Ok(capabilities);
                    }
                }
            }
        }
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
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
                    ("_Gi", '=') => {
                        self.current = ResponseType::Kitty;
                    }
                    _ => (),
                };
                self.data.push(next);
            }
            ResponseType::Kitty => match next {
                '\\' => {
                    let response = if self.data.ends_with(";OK\x1b") { Some(Response::KittySupported) } else { None };
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

                    return Some(Response::Capabilities { sixel });
                }
                _ => self.data.push(next),
            },
        };
        None
    }
}

#[derive(Default)]
enum ResponseType {
    #[default]
    Unknown,
    Kitty,
    Capabilities,
}

enum Response {
    KittySupported,
    Capabilities { sixel: bool },
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;
    use rstest::rstest;

    #[rstest]
    #[case("\x1b_Gi=42;OK\x1b\\\x1b[?c", true, false)]
    #[case("\x1b[?c", false, false)]
    #[case("\x1b[?4c", false, true)]
    #[case("\x1b[?4;42c", false, true)]
    #[case("\x1b[?1337;4;42c", false, true)]
    fn detection(#[case] input: &str, #[case] kitty_local: bool, #[case] sixel: bool) {
        let input = Cursor::new(input);
        let capabilities = TerminalCapabilities::parse_response(input).expect("reading failed");
        assert_eq!(capabilities.kitty_local, kitty_local);
        assert_eq!(capabilities.sixel, sixel);
    }
}
