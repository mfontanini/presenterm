use crate::{
    code::execute::SnippetExecutor,
    commands::{
        listener::{Command, CommandListener},
        speaker_notes::{SpeakerNotesEvent, SpeakerNotesEventPublisher},
    },
    config::{KeyBindingsConfig, SlideTransitionConfig, SlideTransitionStyleConfig},
    markdown::parse::{MarkdownParser, ParseError},
    presentation::{
        Presentation, Slide,
        builder::{BuildError, PresentationBuilder, PresentationBuilderOptions, Themes},
        diff::PresentationDiffer,
    },
    render::{
        ErrorSource, RenderError, RenderResult, TerminalDrawer, TerminalDrawerOptions,
        engine::{MaxSize, RenderEngine, RenderEngineOptions},
        operation::RenderAsyncState,
        properties::WindowSize,
        validate::OverflowValidator,
    },
    resource::Resources,
    terminal::{
        image::printer::{ImagePrinter, ImageRegistry},
        printer::{TerminalCommand, TerminalIo},
        virt::{ImageBehavior, TerminalGrid, VirtualTerminal},
    },
    theme::{ProcessingThemeError, raw::PresentationTheme},
    third_party::ThirdPartyRender,
    transitions::{
        AnimateTransition, AnimationFrame, LinesFrame, TransitionDirection, fade::FadeAnimation,
        slide_horizontal::SlideHorizontalAnimation,
    },
};
use std::{
    collections::HashSet,
    fmt::Display,
    fs,
    io::{self},
    mem,
    ops::Deref,
    path::Path,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct PresenterOptions {
    pub mode: PresentMode,
    pub builder_options: PresentationBuilderOptions,
    pub font_size_fallback: u8,
    pub bindings: KeyBindingsConfig,
    pub validate_overflows: bool,
    pub max_size: MaxSize,
    pub transition: Option<SlideTransitionConfig>,
}

/// A slideshow presenter.
///
/// This type puts everything else together.
pub struct Presenter<'a> {
    default_theme: &'a PresentationTheme,
    listener: CommandListener,
    parser: MarkdownParser<'a>,
    resources: Resources,
    third_party: ThirdPartyRender,
    code_executor: Rc<SnippetExecutor>,
    state: PresenterState,
    slides_with_pending_async_renders: HashSet<usize>,
    image_printer: Arc<ImagePrinter>,
    themes: Themes,
    options: PresenterOptions,
    speaker_notes_event_publisher: Option<SpeakerNotesEventPublisher>,
}

impl<'a> Presenter<'a> {
    /// Construct a new presenter.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        default_theme: &'a PresentationTheme,
        listener: CommandListener,
        parser: MarkdownParser<'a>,
        resources: Resources,
        third_party: ThirdPartyRender,
        code_executor: Rc<SnippetExecutor>,
        themes: Themes,
        image_printer: Arc<ImagePrinter>,
        options: PresenterOptions,
        speaker_notes_event_publisher: Option<SpeakerNotesEventPublisher>,
    ) -> Self {
        Self {
            default_theme,
            listener,
            parser,
            resources,
            third_party,
            code_executor,
            state: PresenterState::Empty,
            slides_with_pending_async_renders: HashSet::new(),
            image_printer,
            themes,
            options,
            speaker_notes_event_publisher,
        }
    }

    /// Run a presentation.
    pub fn present(mut self, path: &Path) -> Result<(), PresentationError> {
        if matches!(self.options.mode, PresentMode::Development) {
            self.resources.watch_presentation_file(path.to_path_buf());
        }
        self.state = PresenterState::Presenting(Presentation::from(vec![]));
        self.try_reload(path, true);

        let drawer_options = TerminalDrawerOptions {
            font_size_fallback: self.options.font_size_fallback,
            max_size: self.options.max_size.clone(),
        };
        let mut drawer = TerminalDrawer::new(self.image_printer.clone(), drawer_options)?;
        loop {
            // Poll async renders once before we draw just in case.
            self.poll_async_renders()?;
            self.render(&mut drawer)?;

            loop {
                if self.poll_async_renders()? {
                    self.render(&mut drawer)?;
                }

                let command = match self.listener.try_next_command()? {
                    Some(command) => command,
                    _ => match self.resources.resources_modified() {
                        true => Command::Reload,
                        false => {
                            if self.check_async_error() {
                                break;
                            }
                            continue;
                        }
                    },
                };
                match self.apply_command(command) {
                    CommandSideEffect::Exit => {
                        self.publish_event(SpeakerNotesEvent::Exit)?;
                        return Ok(());
                    }
                    CommandSideEffect::Suspend => {
                        self.suspend(&mut drawer);
                        break;
                    }
                    CommandSideEffect::Reload => {
                        self.try_reload(path, false);
                        break;
                    }
                    CommandSideEffect::Redraw => {
                        break;
                    }
                    CommandSideEffect::NextSlide => {
                        self.next_slide(&mut drawer)?;
                        break;
                    }
                    CommandSideEffect::PreviousSlide => {
                        self.previous_slide(&mut drawer)?;
                        break;
                    }
                    CommandSideEffect::None => (),
                };
            }
            self.publish_event(SpeakerNotesEvent::GoToSlide {
                slide: self.state.presentation().current_slide_index() as u32 + 1,
            })?;
        }
    }

    fn publish_event(&self, event: SpeakerNotesEvent) -> io::Result<()> {
        if let Some(publisher) = &self.speaker_notes_event_publisher {
            publisher.send(event)?;
        }
        Ok(())
    }

    fn check_async_error(&mut self) -> bool {
        let error_holder = self.state.presentation().state.async_error_holder();
        let error_holder = error_holder.lock().unwrap();
        match error_holder.deref() {
            Some(error) => {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = PresenterState::failure(
                    &error.error,
                    presentation,
                    ErrorSource::Slide(error.slide),
                    FailureMode::Other,
                );
                true
            }
            None => false,
        }
    }

    fn poll_async_renders(&mut self) -> Result<bool, RenderError> {
        if matches!(self.state, PresenterState::Failure { .. }) {
            return Ok(false);
        }
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

    fn render(&mut self, drawer: &mut TerminalDrawer) -> RenderResult {
        let result = match &self.state {
            PresenterState::Presenting(presentation) => {
                drawer.render_operations(presentation.current_slide().iter_visible_operations())
            }
            PresenterState::SlideIndex(presentation) => {
                drawer.render_operations(presentation.current_slide().iter_visible_operations())?;
                drawer.render_operations(presentation.iter_slide_index_operations())
            }
            PresenterState::KeyBindings(presentation) => {
                drawer.render_operations(presentation.current_slide().iter_visible_operations())?;
                drawer.render_operations(presentation.iter_bindings_operations())
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
            Command::Suspend => return CommandSideEffect::Suspend,
            _ => (),
        };
        if matches!(command, Command::Redraw) {
            if !self.is_displaying_other_error() {
                let presentation = mem::take(&mut self.state).into_presentation();
                self.state = self.validate_overflows(presentation);
            }
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
            Command::Next => {
                let current_slide = presentation.current_slide_index();
                if !presentation.jump_next() {
                    false
                } else if presentation.current_slide_index() != current_slide {
                    return CommandSideEffect::NextSlide;
                } else {
                    true
                }
            }
            Command::NextFast => presentation.jump_next_fast(),
            Command::Previous => {
                let current_slide = presentation.current_slide_index();
                if !presentation.jump_previous() {
                    false
                } else if presentation.current_slide_index() != current_slide {
                    return CommandSideEffect::PreviousSlide;
                } else {
                    true
                }
            }
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
            Command::Reload | Command::HardReload | Command::Exit | Command::Suspend | Command::Redraw => {
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
        self.resources.clear_watches();
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
                self.state = PresenterState::failure(e, presentation, ErrorSource::Presentation, FailureMode::Other);
            }
        };
    }

    fn is_displaying_other_error(&self) -> bool {
        matches!(self.state, PresenterState::Failure { mode: FailureMode::Other, .. })
    }

    fn validate_overflows(&self, presentation: Presentation) -> PresenterState {
        if self.options.validate_overflows {
            let dimensions = match WindowSize::current(self.options.font_size_fallback) {
                Ok(dimensions) => dimensions,
                Err(e) => {
                    return PresenterState::failure(e, presentation, ErrorSource::Presentation, FailureMode::Other);
                }
            };
            match OverflowValidator::validate(&presentation, dimensions) {
                Ok(()) => PresenterState::Presenting(presentation),
                Err(e) => PresenterState::failure(e, presentation, ErrorSource::Presentation, FailureMode::Overflow),
            }
        } else {
            PresenterState::Presenting(presentation)
        }
    }

    fn load_presentation(&mut self, path: &Path) -> Result<Presentation, LoadPresentationError> {
        let content = fs::read_to_string(path).map_err(LoadPresentationError::Reading)?;
        let elements = self.parser.parse(&content)?;
        let presentation = PresentationBuilder::new(
            self.default_theme,
            self.resources.clone(),
            &mut self.third_party,
            self.code_executor.clone(),
            &self.themes,
            ImageRegistry::new(self.image_printer.clone()),
            self.options.bindings.clone(),
            self.options.builder_options.clone(),
        )?
        .build(elements)?;
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

    fn suspend(&self, drawer: &mut TerminalDrawer) {
        #[cfg(unix)]
        unsafe {
            drawer.terminal.suspend();
            libc::raise(libc::SIGTSTP);
            drawer.terminal.resume();
        }
    }

    fn next_slide(&mut self, drawer: &mut TerminalDrawer) -> RenderResult {
        let Some(config) = self.options.transition.clone() else {
            return Ok(());
        };
        let options = drawer.render_engine_options();
        let presentation = self.state.presentation_mut();
        let dimensions = WindowSize::current(self.options.font_size_fallback)?;
        presentation.jump_previous();
        let left = Self::virtual_render(presentation.current_slide(), dimensions.clone(), &options)?;
        presentation.jump_next();
        let right = Self::virtual_render(presentation.current_slide(), dimensions.clone(), &options)?;
        let direction = TransitionDirection::Next;
        self.animate_transition(drawer, left, right, direction, dimensions, config)
    }

    fn previous_slide(&mut self, drawer: &mut TerminalDrawer) -> RenderResult {
        let Some(config) = self.options.transition.clone() else {
            return Ok(());
        };
        let options = drawer.render_engine_options();
        let presentation = self.state.presentation_mut();
        let dimensions = WindowSize::current(self.options.font_size_fallback)?;
        presentation.jump_next();
        let right = Self::virtual_render(presentation.current_slide(), dimensions.clone(), &options)?;
        presentation.jump_previous();
        let left = Self::virtual_render(presentation.current_slide(), dimensions.clone(), &options)?;
        let direction = TransitionDirection::Previous;
        self.animate_transition(drawer, left, right, direction, dimensions, config)
    }

    fn animate_transition(
        &mut self,
        drawer: &mut TerminalDrawer,
        left: TerminalGrid,
        right: TerminalGrid,
        direction: TransitionDirection,
        dimensions: WindowSize,
        config: SlideTransitionConfig,
    ) -> RenderResult {
        let first = match &direction {
            TransitionDirection::Next => left.clone(),
            TransitionDirection::Previous => right.clone(),
        };
        match &config.animation {
            SlideTransitionStyleConfig::SlideHorizontal => self.run_animation(
                drawer,
                first,
                SlideHorizontalAnimation::new(left, right, dimensions, direction),
                config,
            ),
            SlideTransitionStyleConfig::Fade => {
                self.run_animation(drawer, first, FadeAnimation::new(left, right, direction), config)
            }
        }
    }

    fn run_animation<T>(
        &mut self,
        drawer: &mut TerminalDrawer,
        first: TerminalGrid,
        animation: T,
        config: SlideTransitionConfig,
    ) -> RenderResult
    where
        T: AnimateTransition,
    {
        let total_time = Duration::from_millis(config.duration_millis as u64);
        let frames: usize = config.frames;
        let total_frames = animation.total_frames();
        let step = total_time / (frames as u32 * 2);
        let mut last_frame_index = 0;
        let mut frame_index = 1;
        // Render the first frame as text to have images as ascii
        Self::render_frame(&LinesFrame::from(&first).build_commands(), drawer)?;
        while frame_index < total_frames {
            let start = Instant::now();
            let frame = animation.build_frame(frame_index, last_frame_index);
            let commands = frame.build_commands();
            Self::render_frame(&commands, drawer)?;

            let elapsed = start.elapsed();
            let sleep_needed = step.saturating_sub(elapsed);
            if sleep_needed.as_millis() > 0 {
                std::thread::sleep(step);
            }
            last_frame_index = frame_index;
            frame_index += total_frames.div_ceil(frames);
        }
        Ok(())
    }

    fn render_frame(commands: &[TerminalCommand<'_>], drawer: &mut TerminalDrawer) -> RenderResult {
        drawer.terminal.execute(&TerminalCommand::BeginUpdate)?;
        for command in commands {
            drawer.terminal.execute(command)?;
        }
        drawer.terminal.execute(&TerminalCommand::EndUpdate)?;
        drawer.terminal.execute(&TerminalCommand::Flush)?;
        Ok(())
    }

    fn virtual_render(
        slide: &Slide,
        dimensions: WindowSize,
        options: &RenderEngineOptions,
    ) -> Result<TerminalGrid, RenderError> {
        let mut term = VirtualTerminal::new(dimensions.clone(), ImageBehavior::PrintAscii);
        let engine = RenderEngine::new(&mut term, dimensions.clone(), options.clone());
        engine.render(slide.iter_visible_operations())?;
        Ok(term.into_contents())
    }
}

enum CommandSideEffect {
    Exit,
    Suspend,
    Redraw,
    Reload,
    NextSlide,
    PreviousSlide,
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
        mode: FailureMode,
    },
}

impl PresenterState {
    pub(crate) fn failure<E: Display>(
        error: E,
        presentation: Presentation,
        source: ErrorSource,
        mode: FailureMode,
    ) -> Self {
        PresenterState::Failure { error: error.to_string(), presentation, source, mode }
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

enum FailureMode {
    Overflow,
    Other,
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

    #[error("processing theme: {0}")]
    ProcessingTheme(#[from] ProcessingThemeError),
}

/// An error during the presentation.
#[derive(thiserror::Error, Debug)]
pub enum PresentationError {
    #[error(transparent)]
    Render(#[from] RenderError),

    #[error("io: {0}")]
    Io(#[from] io::Error),
}
