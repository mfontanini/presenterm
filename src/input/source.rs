use super::{
    fs::PresentationFileWatcher,
    user::{CommandKeyBindings, KeyBindingsValidationError, UserInput},
};
use crate::custom::KeyBindingsConfig;
use serde::Deserialize;
use std::{io, path::PathBuf, time::Duration};
use strum::EnumDiscriminants;

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
    pub fn new<P: Into<PathBuf>>(
        presentation_path: P,
        config: KeyBindingsConfig,
    ) -> Result<Self, KeyBindingsValidationError> {
        let watcher = PresentationFileWatcher::new(presentation_path);
        let bindings = CommandKeyBindings::try_from(config)?;
        Ok(Self { watcher, user_input: UserInput::new(bindings) })
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
#[derive(Clone, Debug, PartialEq, Eq, EnumDiscriminants)]
#[strum_discriminants(derive(Deserialize))]
pub(crate) enum Command {
    /// Redraw the presentation.
    ///
    /// This can happen on terminal resize.
    Redraw,

    /// Go to the next slide.
    NextSlide,

    /// Go to the previous slide.
    PreviousSlide,

    /// Go to the first slide.
    FirstSlide,

    /// Go to the last slide.
    LastSlide,

    /// Go to one particular slide.
    GoToSlide(u32),

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

    /// Toggle the slide index view.
    ToggleSlideIndex,

    /// Hide the currently open modal, if any.
    CloseModal,
}
