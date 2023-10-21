use super::{
    fs::PresentationFileWatcher,
    user::{UserCommand, UserInput},
};
use std::{io, path::PathBuf, time::Duration};

/// The source of commands.
///
/// This expects user commands as well as watches over the presentation file to reload if it that
/// happens.
pub struct CommandSource {
    watcher: PresentationFileWatcher,
    user_input: UserInput,
}

impl CommandSource {
    /// Create a new command source over the given presentation path.
    pub fn new<P: Into<PathBuf>>(presentation_path: P) -> Self {
        let watcher = PresentationFileWatcher::new(presentation_path);
        Self { watcher, user_input: UserInput::default() }
    }

    /// Block until the next command arrives.
    pub fn next_command(&mut self) -> io::Result<Command> {
        loop {
            if let Some(command) = self.try_next_command()? {
                return Ok(command);
            }
        }
    }

    /// Try to get the next command.
    ///
    /// This attempts to get a command and returns `Ok(None)` on timeout.
    pub fn try_next_command(&mut self) -> io::Result<Option<Command>> {
        match self.user_input.poll_next_command(Duration::from_millis(250)) {
            Ok(Some(command)) => {
                return Ok(Some(Command::User(command)));
            }
            Ok(None) => (),
            Err(e) => {
                return Ok(Some(Command::Abort { error: e.to_string() }));
            }
        };
        if self.watcher.has_modifications()? { Ok(Some(Command::ReloadPresentation)) } else { Ok(None) }
    }
}

/// A command.
#[derive(Clone, Debug)]
pub enum Command {
    /// A user input command.
    User(UserCommand),

    /// The presentation has changed and needs to be reloaded.
    ReloadPresentation,

    /// Something bad has happened and we need to abort.
    Abort { error: String },
}
