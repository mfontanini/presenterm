use crate::{
    code::{
        execute::{ExecutionHandle, LanguageSnippetExecutor, ProcessStatus},
        snippet::Snippet,
    },
    markdown::elements::Text,
    render::{
        operation::{
            AsRenderOperations, ImageRenderProperties, Pollable, PollableState, RenderAsync, RenderAsyncStartPolicy,
            RenderOperation, RenderTextProperties,
        },
        properties::WindowSize,
    },
    terminal::image::{
        Image,
        printer::{ImageRegistry, ImageSpec},
    },
    theme::{Alignment, ExecutionStatusBlockStyle, Margin},
};
use std::{
    io::BufRead,
    mem,
    ops::Deref,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) struct RunImageSnippet {
    snippet: Snippet,
    state: Arc<Mutex<State>>,
    image_registry: ImageRegistry,
    colors: ExecutionStatusBlockStyle,
}

impl RunImageSnippet {
    pub(crate) fn new(
        snippet: Snippet,
        executor: LanguageSnippetExecutor,
        image_registry: ImageRegistry,
        colors: ExecutionStatusBlockStyle,
    ) -> Self {
        let state = Arc::new(Mutex::new(State::NotStarted(executor)));
        Self { snippet, image_registry, colors, state }
    }
}

impl RenderAsync for RunImageSnippet {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable {
            state: self.state.clone(),
            snippet: self.snippet.clone(),
            image_registry: self.image_registry.clone(),
        })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        RenderAsyncStartPolicy::Automatic
    }
}

impl AsRenderOperations for RunImageSnippet {
    fn as_render_operations(&self, _dimensions: &WindowSize) -> Vec<RenderOperation> {
        let state = self.state.lock().unwrap();
        match state.deref() {
            State::NotStarted(_) | State::Running(_) => vec![],
            State::Success(image) => {
                vec![RenderOperation::RenderImage(image.clone(), ImageRenderProperties::default())]
            }
            State::Failure(lines) => {
                let mut output = Vec::new();
                for line in lines {
                    output.extend([RenderOperation::RenderText {
                        line: vec![Text::new(line, self.colors.failure_style)].into(),
                        properties: RenderTextProperties {
                            alignment: Alignment::Left { margin: Margin::Percent(25) },
                            ..Default::default()
                        },
                    }]);
                }
                output
            }
        }
    }
}

struct OperationPollable {
    state: Arc<Mutex<State>>,
    snippet: Snippet,
    image_registry: ImageRegistry,
}

impl OperationPollable {
    fn load_image(&self, data: &[u8]) -> Result<Image, String> {
        let image = match image::load_from_memory(data) {
            Ok(image) => image,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        self.image_registry.register(ImageSpec::Generated(image)).map_err(|e| e.to_string())
    }
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        let mut state = self.state.lock().unwrap();
        match state.deref() {
            State::NotStarted(executor) => match executor.execute_async(&self.snippet) {
                Ok(handle) => {
                    *state = State::Running(handle);
                    PollableState::Unmodified
                }
                Err(e) => {
                    *state = State::Failure(e.to_string().lines().map(ToString::to_string).collect());
                    PollableState::Done
                }
            },
            State::Running(handle) => {
                let mut inner = handle.state.lock().unwrap();
                match inner.status {
                    ProcessStatus::Running => PollableState::Unmodified,
                    ProcessStatus::Success => {
                        let data = mem::take(&mut inner.output);
                        drop(inner);

                        match self.load_image(&data) {
                            Ok(image) => {
                                *state = State::Success(image);
                            }
                            Err(e) => {
                                *state = State::Failure(vec![e.to_string()]);
                            }
                        };
                        PollableState::Done
                    }
                    ProcessStatus::Failure => {
                        let mut lines = Vec::new();
                        for line in inner.output.lines() {
                            lines.push(line.unwrap_or_else(|_| String::new()));
                        }
                        drop(inner);

                        *state = State::Failure(lines);
                        PollableState::Done
                    }
                }
            }
            State::Success(_) | State::Failure(_) => PollableState::Done,
        }
    }
}

#[derive(Debug)]
enum State {
    NotStarted(LanguageSnippetExecutor),
    Running(ExecutionHandle),
    Success(Image),
    Failure(Vec<String>),
}
