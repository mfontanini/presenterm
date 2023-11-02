use super::{fs::PresentationFileWatcher, user::UserInput};
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

    /// Try to get the next command.
    ///
    /// This attempts to get a command and returns `Ok(None)` on timeout.
    pub(crate) fn try_next_command(&mut self) -> io::Result<Option<Command>> {
        if let Some(command) = self.user_input.poll_next_command(Duration::from_millis(250))? {
            return Ok(Some(command));
        };
        if self.watcher.has_modifications()? { Ok(Some(Command::Reload)) } else { Ok(None) }
    }
}

/// A command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Redraw the presentation.
    ///
    /// This can happen on terminal resize.
    Redraw,

    /// Jump to the next slide.
    JumpNextSlide,

    /// Jump to the previous slide.
    JumpPreviousSlide,

    /// Jump to the first slide.
    JumpFirstSlide,

    /// Jump to the last slide.
    JumpLastSlide,

    /// Jump to one particular slide.
    JumpSlide(u32),

    /// Render any widgets in the currently visible slide.
    RenderWidgets,

    /// Exit the presentation.
    Exit,

    /// The presentation has changed and needs to be reloaded.
    Reload,

    /// Hard reload the presentation.
    ///
    /// Like [Command::Reload] but also reloads any external resources like images and themes.
    HardReload,
}
