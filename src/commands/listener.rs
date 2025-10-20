use super::{
    keyboard::{CommandKeyBindings, KeyBindingsValidationError, KeyboardListener},
    speaker_notes::{SpeakerNotesEvent, SpeakerNotesEventListener},
};
use crate::{config::KeyBindingsConfig, presenter::PresentationError};
use serde::Deserialize;
use std::time::Duration;
use strum::EnumDiscriminants;

/// A command listener that allows polling all command sources in a single place.
pub struct CommandListener {
    keyboard: KeyboardListener,
    speaker_notes_event_listener: Option<SpeakerNotesEventListener>,
}

impl CommandListener {
    /// Create a new command source over the given presentation path.
    pub fn new(
        config: KeyBindingsConfig,
        speaker_notes_event_listener: Option<SpeakerNotesEventListener>,
    ) -> Result<Self, KeyBindingsValidationError> {
        let bindings = CommandKeyBindings::try_from(config)?;
        Ok(Self { keyboard: KeyboardListener::new(bindings), speaker_notes_event_listener })
    }

    /// Try to get the next command.
    ///
    /// This attempts to get a command and returns `Ok(None)` on timeout.
    pub(crate) fn try_next_command(&mut self) -> Result<Option<Command>, PresentationError> {
        if let Some(receiver) = &self.speaker_notes_event_listener {
            if let Some(msg) = receiver.try_recv()? {
                let command = match msg {
                    SpeakerNotesEvent::GoToSlide { slide, chunk } => Command::GoToSlideChunk { slide, chunk },
                    SpeakerNotesEvent::Exit => Command::Exit,
                };
                return Ok(Some(command));
            }
        }
        match self.keyboard.poll_next_command(Duration::from_millis(100))? {
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

    /// Go to one particular slide and chunk.
    GoToSlideChunk { slide: u32, chunk: u32 },

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

    /// Skip pauses in the current slide.
    SkipPauses,
}
