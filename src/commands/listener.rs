use super::{
    SpeakerNotesCommand,
    keyboard::{CommandKeyBindings, KeyBindingsValidationError, KeyboardListener},
};
use crate::{custom::KeyBindingsConfig, presenter::PresentationError};
use iceoryx2::{port::subscriber::Subscriber, service::ipc::Service};
use serde::Deserialize;
use std::time::Duration;
use strum::EnumDiscriminants;

/// A command listener that allows polling all command sources in a single place.
pub struct CommandListener {
    keyboard: KeyboardListener,
    speaker_notes_event_receiver: Option<Subscriber<Service, SpeakerNotesCommand, ()>>,
}

impl CommandListener {
    /// Create a new command source over the given presentation path.
    pub fn new(
        config: KeyBindingsConfig,
        speaker_notes_event_receiver: Option<Subscriber<Service, SpeakerNotesCommand, ()>>,
    ) -> Result<Self, KeyBindingsValidationError> {
        let bindings = CommandKeyBindings::try_from(config)?;
        Ok(Self { keyboard: KeyboardListener::new(bindings), speaker_notes_event_receiver })
    }

    /// Try to get the next command.
    ///
    /// This attempts to get a command and returns `Ok(None)` on timeout.
    pub(crate) fn try_next_command(&mut self) -> Result<Option<Command>, PresentationError> {
        if let Some(receiver) = self.speaker_notes_event_receiver.as_mut() {
            if let Some(msg) = receiver.receive()? {
                let command = match msg.payload() {
                    SpeakerNotesCommand::GoToSlide(idx) => Command::GoToSlide(*idx),
                    SpeakerNotesCommand::Exit => Command::Exit,
                };
                return Ok(Some(command));
            }
        }
        match self.keyboard.poll_next_command(Duration::from_millis(250))? {
            Some(command) => Ok(Some(command)),
            None => Ok(None),
        }
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

    /// Move forward in the presentation.
    Next,

    /// Move to the next slide fast.
    NextFast,

    /// Move backwards in the presentation.
    Previous,

    /// Move to the previous slide fast.
    PreviousFast,

    /// Go to the first slide.
    FirstSlide,

    /// Go to the last slide.
    LastSlide,

    /// Go to one particular slide.
    GoToSlide(u32),

    /// Render any async render operations in the current slide.
    RenderAsyncOperations,

    /// Exit the presentation.
    Exit,

    /// Suspend the presentation.
    Suspend,

    /// The presentation has changed and needs to be reloaded.
    Reload,

    /// Hard reload the presentation.
    ///
    /// Like [Command::Reload] but also reloads any external resources like images and themes.
    HardReload,

    /// Toggle the slide index view.
    ToggleSlideIndex,

    /// Toggle the key bindings config view.
    ToggleKeyBindingsConfig,

    /// Hide the currently open modal, if any.
    CloseModal,
}
