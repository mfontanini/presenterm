use crate::{
    builder::{BuildError, PresentationBuilder},
    input::{
        source::{Command, CommandSource},
        user::UserCommand,
    },
    markdown::parse::{MarkdownParser, ParseError},
    presentation::Presentation,
    render::{
        draw::{DrawResult, DrawSlideError, Drawer},
        highlighting::CodeHighlighter,
    },
    resource::Resources,
    theme::PresentationTheme,
};
use std::{
    fs,
    io::{self, Stdout},
    path::Path,
};

pub struct SlideShow<'a> {
    default_theme: &'a PresentationTheme,
    commands: CommandSource,
    parser: MarkdownParser<'a>,
    resources: Resources,
    highlighter: CodeHighlighter,
    state: SlideShowState,
}

impl<'a> SlideShow<'a> {
    pub fn new(
        default_theme: &'a PresentationTheme,
        commands: CommandSource,
        parser: MarkdownParser<'a>,
        resources: Resources,
        highlighter: CodeHighlighter,
    ) -> Self {
        Self { default_theme, commands, parser, resources, highlighter, state: SlideShowState::Empty }
    }

    pub fn present(mut self, path: &Path) -> Result<(), SlideShowError> {
        self.state = SlideShowState::Presenting(self.load_presentation(path)?);

        let mut drawer = Drawer::new(io::stdout())?;
        loop {
            self.render(&mut drawer)?;

            loop {
                let command = match self.commands.next_command()? {
                    Command::User(command) => command,
                    Command::ReloadPresentation => {
                        match self.load_presentation(path) {
                            Ok(mut presentation) => {
                                presentation.jump_slide(self.state.presentation().current_slide_index());
                                self.state = SlideShowState::Presenting(presentation)
                            }
                            Err(e) => {
                                self.state = SlideShowState::Failure {
                                    error: e.to_string(),
                                    presentation: self.state.into_presentation(),
                                }
                            }
                        };
                        break;
                    }
                    Command::Abort { error } => return Err(SlideShowError::Fatal(error)),
                };
                match self.apply_user_command(command) {
                    CommandSideEffect::Exit => return Ok(()),
                    CommandSideEffect::Redraw => {
                        break;
                    }
                    CommandSideEffect::None => (),
                };
            }
        }
    }

    fn render(&mut self, drawer: &mut Drawer<Stdout>) -> DrawResult {
        match &self.state {
            SlideShowState::Presenting(presentation) => drawer.render_slide(presentation),
            SlideShowState::Failure { error, .. } => drawer.render_error(error),
            SlideShowState::Empty => panic!("cannot render without state"),
        }
    }

    fn apply_user_command(&mut self, command: UserCommand) -> CommandSideEffect {
        let SlideShowState::Presenting(presentation) = &mut self.state else {
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
        if needs_redraw { CommandSideEffect::Redraw } else { CommandSideEffect::None }
    }

    fn load_presentation(&mut self, path: &Path) -> Result<Presentation, LoadPresentationError> {
        let content = fs::read_to_string(path).map_err(LoadPresentationError::Reading)?;
        let elements = self.parser.parse(&content)?;
        let presentation =
            PresentationBuilder::new(&self.highlighter, self.default_theme, &mut self.resources).build(elements)?;
        Ok(presentation)
    }
}

enum CommandSideEffect {
    Exit,
    Redraw,
    None,
}

enum SlideShowState {
    Empty,
    Presenting(Presentation),
    Failure { error: String, presentation: Presentation },
}

impl SlideShowState {
    fn presentation(&self) -> &Presentation {
        match self {
            Self::Presenting(presentation) => presentation,
            Self::Failure { presentation, .. } => presentation,
            Self::Empty => panic!("state is empty"),
        }
    }

    fn into_presentation(self) -> Presentation {
        match self {
            Self::Presenting(presentation) => presentation,
            Self::Failure { presentation, .. } => presentation,
            Self::Empty => panic!("state is empty"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoadPresentationError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("reading presentation: {0}")]
    Reading(io::Error),

    #[error(transparent)]
    Processing(#[from] BuildError),
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
