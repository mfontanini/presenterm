pub(crate) mod keyboard;
pub(crate) mod listener;

#[derive(Debug)]
#[repr(C)]
pub enum SpeakerNotesCommand {
    GoToSlide(u32),
    Exit,
}
