use super::{
    fs::PresentationFileWatcher,
    user::{UserCommand, UserInput},
};
use std::{io, path::PathBuf, time::Duration};

pub struct CommandSource {
    watcher: PresentationFileWatcher,
    user_input: UserInput,
}

impl CommandSource {
    pub fn new<P: Into<PathBuf>>(presentation_path: P) -> Self {
        let watcher = PresentationFileWatcher::new(presentation_path);
        Self { watcher, user_input: UserInput::default() }
    }

    pub fn next_command(&mut self) -> io::Result<Command> {
        loop {
            match self.user_input.poll_next_command(Duration::from_millis(250)) {
                Ok(Some(command)) => {
                    return Ok(Command::User(command));
                }
                Ok(None) => (),
                Err(e) => {
                    return Ok(Command::Abort { error: e.to_string() });
                }
            };
            if self.watcher.has_modifications()? {
                return Ok(Command::ReloadPresentation);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    User(UserCommand),
    ReloadPresentation,
    Abort { error: String },
}
