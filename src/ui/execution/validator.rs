use crate::{
    code::{
        execute::{ExecutionHandle, LanguageSnippetExecutor, ProcessStatus},
        snippet::{ExpectedSnippetExecutionResult, Snippet},
    },
    render::operation::{
        AsRenderOperations, Pollable, PollableState, RenderAsync, RenderAsyncStartPolicy, RenderOperation,
    },
};
use std::{
    mem,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) struct ValidateSnippetOperation {
    snippet: Snippet,
    executor: LanguageSnippetExecutor,
    state: Arc<Mutex<State>>,
}

impl ValidateSnippetOperation {
    pub(crate) fn new(snippet: Snippet, executor: LanguageSnippetExecutor) -> Self {
        Self { snippet, executor, state: Default::default() }
    }
}

impl AsRenderOperations for ValidateSnippetOperation {
    fn as_render_operations(&self, _dimensions: &crate::WindowSize) -> Vec<RenderOperation> {
        vec![]
    }
}

impl RenderAsync for ValidateSnippetOperation {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable {
            snippet: self.snippet.clone(),
            executor: self.executor.clone(),
            state: self.state.clone(),
        })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        RenderAsyncStartPolicy::Automatic
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Initial,
    Running(ExecutionHandle),
    Done(PollableState),
}

struct OperationPollable {
    snippet: Snippet,
    executor: LanguageSnippetExecutor,
    state: Arc<Mutex<State>>,
}

impl OperationPollable {
    fn success_to_pollable_state(&self) -> PollableState {
        match self.snippet.attributes.expected_execution_result {
            ExpectedSnippetExecutionResult::Success => PollableState::Done,
            ExpectedSnippetExecutionResult::Failure => {
                PollableState::Failed { error: "expected snippet to fail but it succeeded".into() }
            }
        }
    }

    fn error_to_pollable_state<S: Into<String>>(&self, error: S) -> PollableState {
        match self.snippet.attributes.expected_execution_result {
            ExpectedSnippetExecutionResult::Success => PollableState::Failed { error: error.into() },
            ExpectedSnippetExecutionResult::Failure => PollableState::Done,
        }
    }
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        let mut state = self.state.lock().expect("lock poisoned");
        let next_state = match mem::take(state.deref_mut()) {
            State::Initial => match self.executor.execute_async(&self.snippet) {
                Ok(handle) => State::Running(handle),
                Err(e) => State::Done(self.error_to_pollable_state(e.to_string())),
            },
            State::Running(handle) => {
                let state = handle.state.lock().expect("lock poisoned");
                match state.status {
                    ProcessStatus::Running => {
                        drop(state);
                        State::Running(handle)
                    }
                    ProcessStatus::Success => State::Done(self.success_to_pollable_state()),
                    ProcessStatus::Failure => {
                        State::Done(self.error_to_pollable_state(String::from_utf8_lossy(&state.output)))
                    }
                }
            }
            State::Done(output) => State::Done(output),
        };
        *state = next_state;
        match &*state {
            State::Initial | State::Running(_) => PollableState::Unmodified,
            State::Done(output) => output.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{
        execute::SnippetExecutor,
        snippet::{SnippetAttributes, SnippetLanguage},
    };
    use rstest::rstest;

    #[rstest]
    #[case::success("fn main() { println!(\"hi\"); }", ExpectedSnippetExecutionResult::Success)]
    #[case::failure("fn main() ", ExpectedSnippetExecutionResult::Failure)]
    fn expectation_matches(#[case] contents: &str, #[case] expected_execution_result: ExpectedSnippetExecutionResult) {
        let snippet = Snippet {
            contents: contents.into(),
            language: SnippetLanguage::Rust,
            attributes: SnippetAttributes { expected_execution_result, ..Default::default() },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let state = Arc::new(Mutex::new(State::default()));
        let mut pollable =
            OperationPollable { snippet: snippet.clone(), executor: executor.clone(), state: state.clone() };
        loop {
            match pollable.poll() {
                PollableState::Unmodified | PollableState::Modified => continue,
                PollableState::Done => break,
                PollableState::Failed { error } => panic!("finished with error: {error}"),
            }
        }
        let mut pollable = OperationPollable { snippet, executor, state: state.clone() };
        assert!(matches!(pollable.poll(), PollableState::Done), "different pollable returned different");
    }

    #[rstest]
    #[case::success("fn main() { println!(\"hi\"); }", ExpectedSnippetExecutionResult::Failure)]
    #[case::failure("fn main() ", ExpectedSnippetExecutionResult::Success)]
    fn expect_does_not_match(
        #[case] contents: &str,
        #[case] expected_execution_result: ExpectedSnippetExecutionResult,
    ) {
        let snippet = Snippet {
            contents: contents.into(),
            language: SnippetLanguage::Rust,
            attributes: SnippetAttributes { expected_execution_result, ..Default::default() },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let state = Arc::new(Mutex::new(State::default()));
        let mut pollable =
            OperationPollable { snippet: snippet.clone(), executor: executor.clone(), state: state.clone() };
        loop {
            match pollable.poll() {
                PollableState::Unmodified | PollableState::Modified => continue,
                PollableState::Done => panic!("finished successfully"),
                PollableState::Failed { .. } => break,
            }
        }
        let mut pollable = OperationPollable { snippet, executor, state: state.clone() };
        assert!(matches!(pollable.poll(), PollableState::Failed { .. }), "different pollable returned different");
    }
}
