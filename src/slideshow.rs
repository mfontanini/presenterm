use crate::{
    input::{
        source::{Command, CommandSource},
        user::UserCommand,
    },
    markdown::{
        parse::{MarkdownParser, ParseError},
        process::{MarkdownProcessor, ProcessError},
    },
    presentation::Presentation,
    render::{
        draw::{DrawSlideError, Drawer},
        highlighting::CodeHighlighter,
    },
    resource::Resources,
    theme::PresentationTheme,
};
use std::{fs, io, path::Path};

pub struct SlideShow<'a> {
    theme: PresentationTheme,
    commands: CommandSource,
    parser: MarkdownParser<'a>,
    resources: Resources,
    highlighter: CodeHighlighter,
    state: SlideShowState,
}

impl<'a> SlideShow<'a> {
    pub fn new(
        theme: PresentationTheme,
        commands: CommandSource,
        parser: MarkdownParser<'a>,
        resources: Resources,
        highlighter: CodeHighlighter,
    ) -> Self {
        Self { theme, commands, parser, resources, highlighter, state: SlideShowState::RenderError("".to_string()) }
    }

    pub fn present(mut self, path: &Path) -> Result<(), SlideShowError> {
        self.state = SlideShowState::RenderSlide(self.load_presentation(path)?);

        let mut drawer = Drawer::new(io::stdout())?;
        let mut current_slide = 0;
        loop {
            match &self.state {
                SlideShowState::RenderSlide(presentation) => {
                    drawer.render_slide(&self.theme, presentation)?;
                    current_slide = presentation.current_slide_index()
                }
                SlideShowState::RenderError(error) => drawer.render_error(error)?,
            };

            loop {
                let command = match self.commands.next_command()? {
                    Command::User(command) => command,
                    Command::ReloadPresentation => {
                        match self.load_presentation(path) {
                            Ok(mut presentation) => {
                                presentation.jump_slide(current_slide);
                                self.state = SlideShowState::RenderSlide(presentation)
                            }
                            Err(e) => {
                                self.state = SlideShowState::RenderError(e.to_string());
                            }
                        };
                        break;
                    }
                    Command::Abort { error } => return Err(SlideShowError::Fatal(error)),
                };
                match self.apply_user_command(command) {
                    CommandSideEffect::Exit => return Ok(()),
                    CommandSideEffect::Redraw => break,
                    CommandSideEffect::None => (),
                };
            }
        }
    }

    fn apply_user_command(&mut self, command: UserCommand) -> CommandSideEffect {
        let SlideShowState::RenderSlide(presentation) = &mut self.state else {
            return CommandSideEffect::None;
        };
        let needs_redraw = match command {
            UserCommand::Redraw => true,
            UserCommand::JumpNextSlide => presentation.jump_next_slide(),
            UserCommand::JumpPreviousSlide => presentation.jump_previous_slide(),
            UserCommand::JumpFirstSlide => presentation.jump_first_slide(),
            UserCommand::JumpLastSlide => presentation.jump_last_slide(),
            UserCommand::JumpSlide(number) => presentation.jump_slide(number.saturating_sub(1) as usize),
            UserCommand::Exit => return CommandSideEffect::Exit,
        };
        if needs_redraw {
            CommandSideEffect::Redraw
        } else {
            CommandSideEffect::None
        }
    }

    fn load_presentation(&mut self, path: &Path) -> Result<Presentation, LoadPresentationError> {
        let content = fs::read_to_string(path).map_err(LoadPresentationError::Reading)?;
        let elements = self.parser.parse(&content)?;
        let slides = MarkdownProcessor::new(&self.highlighter, &self.theme, &mut self.resources).transform(elements)?;
        Ok(Presentation::new(slides))
    }
}

enum CommandSideEffect {
    Exit,
    Redraw,
    None,
}

enum SlideShowState {
    RenderSlide(Presentation),
    RenderError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum LoadPresentationError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("reading presentation: {0}")]
    Reading(io::Error),

    #[error(transparent)]
    Processing(#[from] ProcessError),
}

#[derive(thiserror::Error, Debug)]
pub enum SlideShowError {
    #[error(transparent)]
    Draw(#[from] DrawSlideError),

    #[error(transparent)]
    LoadPresentationError(#[from] LoadPresentationError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("fatal error: {0}")]
    Fatal(String),
}
