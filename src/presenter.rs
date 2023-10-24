use crate::{
    builder::{BuildError, PresentationBuilder},
    diff::PresentationDiffer,
    input::{
        source::{Command, CommandSource},
        user::UserCommand,
    },
    markdown::parse::{MarkdownParser, ParseError},
    presentation::Presentation,
    render::{
        draw::{RenderError, RenderResult, TerminalDrawer},
        highlighting::CodeHighlighter,
    },
    resource::Resources,
    theme::PresentationTheme,
};
use std::{
    collections::HashSet,
    fs,
    io::{self, Stdout},
    mem,
    path::Path,
};

/// A slideshow presenter.
///
/// This type puts everything else together.
pub struct Presenter<'a> {
    default_theme: &'a PresentationTheme,
    default_highlighter: CodeHighlighter,
    commands: CommandSource,
    parser: MarkdownParser<'a>,
    resources: Resources,
    mode: PresentMode,
    state: PresenterState,
    slides_with_pending_widgets: HashSet<usize>,
}

impl<'a> Presenter<'a> {
    /// Construct a new presenter.
    pub fn new(
        default_theme: &'a PresentationTheme,
        default_highlighter: CodeHighlighter,
        commands: CommandSource,
        parser: MarkdownParser<'a>,
        resources: Resources,
        mode: PresentMode,
    ) -> Self {
        Self {
            default_theme,
            default_highlighter,
            commands,
            parser,
            resources,
            mode,
            state: PresenterState::Empty,
            slides_with_pending_widgets: HashSet::new(),
        }
    }

    /// Run a presentation.
    pub fn present(mut self, path: &Path) -> Result<(), PresentationError> {
        self.state = PresenterState::Presenting(self.load_presentation(path)?);

        let mut drawer = TerminalDrawer::new(io::stdout())?;
        loop {
            self.render(&mut drawer)?;
            self.update_widgets(&mut drawer)?;

            loop {
                self.update_widgets(&mut drawer)?;
                let Some(command) = self.commands.try_next_command()? else {
                    continue;
                };
                let command = match command {
                    Command::User(command) => command,
                    Command::ReloadPresentation => {
                        self.try_reload(path);
                        break;
                    }
                    Command::Abort { error } => return Err(PresentationError::Fatal(error)),
                };
                match self.apply_user_command(command) {
                    CommandSideEffect::Exit => return Ok(()),
                    CommandSideEffect::Redraw => {
                        break;
                    }
                    CommandSideEffect::PollWidgets => {
                        self.slides_with_pending_widgets.insert(self.state.presentation().current_slide_index());
                    }
                    CommandSideEffect::None => (),
                };
            }
        }
    }

    fn update_widgets(&mut self, drawer: &mut TerminalDrawer<Stdout>) -> RenderResult {
        let current_index = self.state.presentation().current_slide_index();
        if self.slides_with_pending_widgets.contains(&current_index) {
            self.render(drawer)?;
            if self.state.presentation_mut().widgets_rendered() {
                // Render one last time just in case it _just_ rendered
                self.render(drawer)?;
                self.slides_with_pending_widgets.remove(&current_index);
            }
        }
        Ok(())
    }

    fn render(&mut self, drawer: &mut TerminalDrawer<Stdout>) -> RenderResult {
        let result = match &self.state {
            PresenterState::Presenting(presentation) => drawer.render_slide(presentation),
            PresenterState::Failure { error, .. } => drawer.render_error(error),
            PresenterState::Empty => panic!("cannot render without state"),
        };
        // If the screen is too small, simply ignore this. Eventually the user will resize the
        // screen.
        if matches!(result, Err(RenderError::TerminalTooSmall)) { Ok(()) } else { result }
    }

    fn apply_user_command(&mut self, command: UserCommand) -> CommandSideEffect {
        // This one always happens no matter our state.
        if matches!(command, UserCommand::Exit) {
            return CommandSideEffect::Exit;
        }
        let PresenterState::Presenting(presentation) = &mut self.state else {
            return CommandSideEffect::None;
        };
        let needs_redraw = match command {
            UserCommand::Redraw => true,
            UserCommand::JumpNextSlide => presentation.jump_next_slide(),
            UserCommand::JumpPreviousSlide => presentation.jump_previous_slide(),
            UserCommand::JumpFirstSlide => presentation.jump_first_slide(),
            UserCommand::JumpLastSlide => presentation.jump_last_slide(),
            UserCommand::JumpSlide(number) => presentation.jump_slide(number.saturating_sub(1) as usize),
            UserCommand::RenderWidgets => {
                if presentation.render_slide_widgets() {
                    self.slides_with_pending_widgets.insert(self.state.presentation().current_slide_index());
                    return CommandSideEffect::PollWidgets;
                } else {
                    return CommandSideEffect::None;
                }
            }
            UserCommand::Exit => return CommandSideEffect::Exit,
        };
        if needs_redraw { CommandSideEffect::Redraw } else { CommandSideEffect::None }
    }

    fn try_reload(&mut self, path: &Path) {
        if matches!(self.mode, PresentMode::Presentation) {
            return;
        }
        self.slides_with_pending_widgets.clear();
        match self.load_presentation(path) {
            Ok(mut presentation) => {
                let current = self.state.presentation();
                if let Some(modification) = PresentationDiffer::find_first_modification(current, &presentation) {
                    presentation.jump_slide(modification.slide_index);
                    presentation.jump_chunk(modification.chunk_index);
                } else {
                    presentation.jump_slide(current.current_slide_index());
                    presentation.jump_chunk(current.current_chunk());
                }
                self.state = PresenterState::Presenting(presentation)
            }
            Err(e) => {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = PresenterState::Failure { error: e.to_string(), presentation }
            }
        };
    }

    fn load_presentation(&mut self, path: &Path) -> Result<Presentation, LoadPresentationError> {
        let content = fs::read_to_string(path).map_err(LoadPresentationError::Reading)?;
        let elements = self.parser.parse(&content)?;
        let presentation =
            PresentationBuilder::new(self.default_highlighter.clone(), self.default_theme, &mut self.resources)
                .build(elements)?;
        Ok(presentation)
    }
}

enum CommandSideEffect {
    Exit,
    Redraw,
    PollWidgets,
    None,
}

#[derive(Default)]
enum PresenterState {
    #[default]
    Empty,
    Presenting(Presentation),
    Failure {
        error: String,
        presentation: Presentation,
    },
}

impl PresenterState {
    fn presentation(&self) -> &Presentation {
        match self {
            Self::Presenting(presentation) => presentation,
            Self::Failure { presentation, .. } => presentation,
            Self::Empty => panic!("state is empty"),
        }
    }

    fn presentation_mut(&mut self) -> &mut Presentation {
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

/// This presentation mode.
pub enum PresentMode {
    /// We are developing the presentation so we want live reloads when the input changes.
    Development,

    /// This is a live presentation so we don't want hot reloading.
    Presentation,
}

/// An error when loading a presentation.
#[derive(thiserror::Error, Debug)]
pub enum LoadPresentationError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("reading presentation: {0}")]
    Reading(io::Error),

    #[error(transparent)]
    Processing(#[from] BuildError),
}

/// An error during the presentation.
#[derive(thiserror::Error, Debug)]
pub enum PresentationError {
    #[error(transparent)]
    Render(#[from] RenderError),

    #[error(transparent)]
    LoadPresentationError(#[from] LoadPresentationError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("fatal error: {0}")]
    Fatal(String),
}
