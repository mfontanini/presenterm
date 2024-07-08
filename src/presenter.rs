use crate::{
    custom::KeyBindingsConfig,
    diff::PresentationDiffer,
    execute::CodeExecutor,
    export::ImageReplacer,
    input::source::{Command, CommandSource},
    markdown::parse::{MarkdownParser, ParseError},
    media::{printer::ImagePrinter, register::ImageRegistry},
    presentation::{Presentation, RenderAsyncState},
    processing::builder::{BuildError, PresentationBuilder, PresentationBuilderOptions, Themes},
    render::{
        draw::{ErrorSource, RenderError, RenderResult, TerminalDrawer},
        properties::WindowSize,
        validate::OverflowValidator,
    },
    resource::Resources,
    theme::PresentationTheme,
    third_party::ThirdPartyRender,
};
use std::{
    collections::HashSet,
    fmt::Display,
    fs,
    io::{self, Stdout},
    mem,
    ops::Deref,
    path::Path,
    rc::Rc,
    sync::Arc,
};

pub struct PresenterOptions {
    pub mode: PresentMode,
    pub builder_options: PresentationBuilderOptions,
    pub font_size_fallback: u8,
    pub bindings: KeyBindingsConfig,
    pub validate_overflows: bool,
}

/// A slideshow presenter.
///
/// This type puts everything else together.
pub struct Presenter<'a> {
    default_theme: &'a PresentationTheme,
    commands: CommandSource,
    parser: MarkdownParser<'a>,
    resources: Resources,
    third_party: ThirdPartyRender,
    code_executor: Rc<CodeExecutor>,
    state: PresenterState,
    slides_with_pending_async_renders: HashSet<usize>,
    image_printer: Arc<ImagePrinter>,
    themes: Themes,
    options: PresenterOptions,
}

impl<'a> Presenter<'a> {
    /// Construct a new presenter.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        default_theme: &'a PresentationTheme,
        commands: CommandSource,
        parser: MarkdownParser<'a>,
        resources: Resources,
        third_party: ThirdPartyRender,
        code_executor: Rc<CodeExecutor>,
        themes: Themes,
        image_printer: Arc<ImagePrinter>,
        options: PresenterOptions,
    ) -> Self {
        Self {
            default_theme,
            commands,
            parser,
            resources,
            third_party,
            code_executor,
            state: PresenterState::Empty,
            slides_with_pending_async_renders: HashSet::new(),
            image_printer,
            themes,
            options,
        }
    }

    /// Run a presentation.
    pub fn present(mut self, path: &Path) -> Result<(), PresentationError> {
        self.state = PresenterState::Presenting(Presentation::from(vec![]));
        self.try_reload(path, true);

        let mut drawer =
            TerminalDrawer::new(io::stdout(), self.image_printer.clone(), self.options.font_size_fallback)?;
        loop {
            // Poll async renders once before we draw just in case.
            self.poll_async_renders()?;
            self.render(&mut drawer)?;

            loop {
                if self.poll_async_renders()? {
                    self.render(&mut drawer)?;
                }
                let Some(command) = self.commands.try_next_command()? else {
                    if self.check_async_error() {
                        break;
                    }
                    continue;
                };
                match self.apply_command(command) {
                    CommandSideEffect::Exit => return Ok(()),
                    CommandSideEffect::Reload => {
                        self.try_reload(path, false);
                        break;
                    }
                    CommandSideEffect::Redraw => {
                        break;
                    }
                    CommandSideEffect::None => (),
                };
            }
        }
    }

    fn check_async_error(&mut self) -> bool {
        let error_holder = self.state.presentation().state.async_error_holder();
        let error_holder = error_holder.lock().unwrap();
        match error_holder.deref() {
            Some(error) => {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = PresenterState::failure(&error.error, presentation, ErrorSource::Slide(error.slide));
                true
            }
            None => false,
        }
    }

    fn poll_async_renders(&mut self) -> Result<bool, RenderError> {
        let current_index = self.state.presentation().current_slide_index();
        if self.slides_with_pending_async_renders.contains(&current_index) {
            let state = self.state.presentation_mut().poll_slide_async_renders();
            match state {
                RenderAsyncState::NotStarted | RenderAsyncState::Rendering { modified: false } => (),
                RenderAsyncState::Rendering { modified: true } => {
                    return Ok(true);
                }
                RenderAsyncState::Rendered | RenderAsyncState::JustFinishedRendering => {
                    self.slides_with_pending_async_renders.remove(&current_index);
                    return Ok(true);
                }
            };
        }
        Ok(false)
    }

    fn render(&mut self, drawer: &mut TerminalDrawer<Stdout>) -> RenderResult {
        let result = match &self.state {
            PresenterState::Presenting(presentation) => drawer.render_slide(presentation),
            PresenterState::SlideIndex(presentation) => {
                drawer.render_slide(presentation)?;
                drawer.render_slide_index(presentation)
            }
            PresenterState::KeyBindings(presentation) => {
                drawer.render_slide(presentation)?;
                drawer.render_key_bindings(presentation)
            }
            PresenterState::Failure { error, source, .. } => drawer.render_error(error, source),
            PresenterState::Empty => panic!("cannot render without state"),
        };
        // If the screen is too small, simply ignore this. Eventually the user will resize the
        // screen.
        if matches!(result, Err(RenderError::TerminalTooSmall)) { Ok(()) } else { result }
    }

    fn apply_command(&mut self, command: Command) -> CommandSideEffect {
        // These ones always happens no matter our state.
        match command {
            Command::Reload => {
                return CommandSideEffect::Reload;
            }
            Command::HardReload => {
                if matches!(self.options.mode, PresentMode::Development) {
                    self.resources.clear();
                }
                return CommandSideEffect::Reload;
            }
            Command::Exit => return CommandSideEffect::Exit,
            _ => (),
        };
        if matches!(command, Command::Redraw) {
            let presentation = mem::take(&mut self.state).into_presentation();
            self.state = self.validate_overflows(presentation);
            return CommandSideEffect::Redraw;
        }

        // Now apply the commands that require a presentation.
        let presentation = match &mut self.state {
            PresenterState::Presenting(presentation)
            | PresenterState::SlideIndex(presentation)
            | PresenterState::KeyBindings(presentation) => presentation,
            _ => {
                return CommandSideEffect::None;
            }
        };
        let needs_redraw = match command {
            Command::Next => presentation.jump_next(),
            Command::NextFast => presentation.jump_next_fast(),
            Command::Previous => presentation.jump_previous(),
            Command::PreviousFast => presentation.jump_previous_fast(),
            Command::FirstSlide => presentation.jump_first_slide(),
            Command::LastSlide => presentation.jump_last_slide(),
            Command::GoToSlide(number) => presentation.go_to_slide(number.saturating_sub(1) as usize),
            Command::RenderAsyncOperations => {
                if presentation.trigger_slide_async_renders() {
                    self.slides_with_pending_async_renders.insert(self.state.presentation().current_slide_index());
                    return CommandSideEffect::Redraw;
                } else {
                    return CommandSideEffect::None;
                }
            }
            Command::ToggleSlideIndex => {
                self.toggle_slide_index();
                true
            }
            Command::ToggleKeyBindingsConfig => {
                self.toggle_key_bindings();
                true
            }
            Command::CloseModal => {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = PresenterState::Presenting(presentation);
                true
            }
            // These are handled above as they don't require the presentation
            Command::Reload | Command::HardReload | Command::Exit | Command::Redraw => {
                panic!("unreachable commands")
            }
        };
        if needs_redraw { CommandSideEffect::Redraw } else { CommandSideEffect::None }
    }

    fn try_reload(&mut self, path: &Path, force: bool) {
        if matches!(self.options.mode, PresentMode::Presentation) && !force {
            return;
        }
        self.slides_with_pending_async_renders.clear();
        match self.load_presentation(path) {
            Ok(mut presentation) => {
                let current = self.state.presentation();
                if let Some(modification) = PresentationDiffer::find_first_modification(current, &presentation) {
                    presentation.go_to_slide(modification.slide_index);
                    presentation.jump_chunk(modification.chunk_index);
                } else {
                    presentation.go_to_slide(current.current_slide_index());
                    presentation.jump_chunk(current.current_chunk());
                }
                self.slides_with_pending_async_renders = presentation.slides_with_async_renders().into_iter().collect();
                self.state = self.validate_overflows(presentation);
            }
            Err(e) => {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = PresenterState::failure(e, presentation, ErrorSource::Presentation);
            }
        };
    }

    fn validate_overflows(&self, presentation: Presentation) -> PresenterState {
        if self.options.validate_overflows {
            let dimensions = match WindowSize::current(self.options.font_size_fallback) {
                Ok(dimensions) => dimensions,
                Err(e) => return PresenterState::failure(e, presentation, ErrorSource::Presentation),
            };
            match OverflowValidator::validate(&presentation, dimensions) {
                Ok(()) => PresenterState::Presenting(presentation),
                Err(e) => PresenterState::failure(e, presentation, ErrorSource::Presentation),
            }
        } else {
            PresenterState::Presenting(presentation)
        }
    }

    fn load_presentation(&mut self, path: &Path) -> Result<Presentation, LoadPresentationError> {
        let content = fs::read_to_string(path).map_err(LoadPresentationError::Reading)?;
        let elements = self.parser.parse(&content)?;
        let export_mode = matches!(self.options.mode, PresentMode::Export);
        let mut presentation = PresentationBuilder::new(
            self.default_theme,
            &mut self.resources,
            &mut self.third_party,
            self.code_executor.clone(),
            &self.themes,
            ImageRegistry(self.image_printer.clone()),
            self.options.bindings.clone(),
            self.options.builder_options.clone(),
        )
        .build(elements)?;
        if export_mode {
            ImageReplacer::default().replace_presentation_images(&mut presentation);
        }

        Ok(presentation)
    }

    fn toggle_slide_index(&mut self) {
        let state = mem::take(&mut self.state);
        match state {
            PresenterState::Presenting(presentation) | PresenterState::KeyBindings(presentation) => {
                self.state = PresenterState::SlideIndex(presentation)
            }
            PresenterState::SlideIndex(presentation) => self.state = PresenterState::Presenting(presentation),
            other => self.state = other,
        }
    }

    fn toggle_key_bindings(&mut self) {
        let state = mem::take(&mut self.state);
        match state {
            PresenterState::Presenting(presentation) | PresenterState::SlideIndex(presentation) => {
                self.state = PresenterState::KeyBindings(presentation)
            }
            PresenterState::KeyBindings(presentation) => self.state = PresenterState::Presenting(presentation),
            other => self.state = other,
        }
    }
}

enum CommandSideEffect {
    Exit,
    Redraw,
    Reload,
    None,
}

#[derive(Default)]
enum PresenterState {
    #[default]
    Empty,
    Presenting(Presentation),
    SlideIndex(Presentation),
    KeyBindings(Presentation),
    Failure {
        error: String,
        presentation: Presentation,
        source: ErrorSource,
    },
}

impl PresenterState {
    pub(crate) fn failure<E: Display>(error: E, presentation: Presentation, source: ErrorSource) -> Self {
        PresenterState::Failure { error: error.to_string(), presentation, source }
    }

    fn presentation(&self) -> &Presentation {
        match self {
            Self::Presenting(presentation)
            | Self::SlideIndex(presentation)
            | Self::KeyBindings(presentation)
            | Self::Failure { presentation, .. } => presentation,
            Self::Empty => panic!("state is empty"),
        }
    }

    fn presentation_mut(&mut self) -> &mut Presentation {
        match self {
            Self::Presenting(presentation)
            | Self::SlideIndex(presentation)
            | Self::KeyBindings(presentation)
            | Self::Failure { presentation, .. } => presentation,
            Self::Empty => panic!("state is empty"),
        }
    }

    fn into_presentation(self) -> Presentation {
        match self {
            Self::Presenting(presentation)
            | Self::SlideIndex(presentation)
            | Self::KeyBindings(presentation)
            | Self::Failure { presentation, .. } => presentation,
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

    /// We are running a presentation that's being consumed by `presenterm-export`.
    Export,
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
    LoadPresentation(#[from] LoadPresentationError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("fatal error: {0}")]
    Fatal(String),
}
