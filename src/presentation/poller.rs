use crate::render::operation::{Pollable, PollableState};
use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    thread,
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(crate) struct Poller {
    sender: Sender<PollerCommand>,
    receiver: Receiver<PollableEffect>,
}

impl Poller {
    pub(crate) fn launch() -> Self {
        let (command_sender, command_receiver) = channel();
        let (effect_sender, effect_receiver) = channel();
        let worker = PollerWorker::new(command_receiver, effect_sender);
        thread::spawn(move || {
            worker.run();
        });
        Self { sender: command_sender, receiver: effect_receiver }
    }

    pub(crate) fn send(&self, command: PollerCommand) {
        let _ = self.sender.send(command);
    }

    pub(crate) fn next_effect(&mut self) -> Option<PollableEffect> {
        self.receiver.try_recv().ok()
    }
}

/// An effect caused by a pollable.
#[derive(Clone)]
pub(crate) enum PollableEffect {
    /// Refresh the given slide.
    RefreshSlide(usize),

    /// Display an error for the given slide.
    DisplayError { slide: usize, error: String },
}

/// A poller command.
pub(crate) enum PollerCommand {
    /// Start polling a pollable that's positioned in the given slide.
    Poll { pollable: Box<dyn Pollable>, slide: usize },

    /// Reset all pollables.
    Reset,
}

struct PollerWorker {
    receiver: Receiver<PollerCommand>,
    sender: Sender<PollableEffect>,
    pollables: Vec<(Box<dyn Pollable>, usize)>,
}

impl PollerWorker {
    fn new(receiver: Receiver<PollerCommand>, sender: Sender<PollableEffect>) -> Self {
        Self { receiver, sender, pollables: Default::default() }
    }

    fn run(mut self) {
        loop {
            match self.receiver.recv_timeout(POLL_INTERVAL) {
                Ok(command) => self.process_command(command),
                // TODO don't loop forever.
                Err(RecvTimeoutError::Timeout) => self.poll(),
                Err(RecvTimeoutError::Disconnected) => break,
            };
        }
    }

    fn process_command(&mut self, command: PollerCommand) {
        match command {
            PollerCommand::Poll { mut pollable, slide } => {
                // Poll and only insert if it's still running.
                match pollable.poll() {
                    PollableState::Unmodified | PollableState::Modified => {
                        self.pollables.push((pollable, slide));
                    }
                    PollableState::Done => {
                        let _ = self.sender.send(PollableEffect::RefreshSlide(slide));
                    }
                    PollableState::Failed { error } => {
                        let _ = self.sender.send(PollableEffect::DisplayError { slide, error });
                    }
                };
            }
            PollerCommand::Reset => self.pollables.clear(),
        }
    }

    fn poll(&mut self) {
        let mut removables = Vec::new();
        for (index, (pollable, slide)) in self.pollables.iter_mut().enumerate() {
            let slide = *slide;
            let (effect, remove) = match pollable.poll() {
                PollableState::Unmodified => (None, false),
                PollableState::Modified => (Some(PollableEffect::RefreshSlide(slide)), false),
                PollableState::Done => (Some(PollableEffect::RefreshSlide(slide)), true),
                PollableState::Failed { error } => (Some(PollableEffect::DisplayError { slide, error }), true),
            };
            if let Some(effect) = effect {
                let _ = self.sender.send(effect);
            }
            if remove {
                removables.push(index);
            }
        }
        // Walk back and swap remove to avoid invalidating indexes.
        for index in removables.iter().rev() {
            self.pollables.swap_remove(*index);
        }
    }
}
