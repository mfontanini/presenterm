#[derive(Debug)]
#[repr(C)]
pub enum SpeakerNotesCommand {
    GoToSlide(u32),
    Exit,
}
