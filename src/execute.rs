//! Code execution.

use crate::{
    custom::LanguageSnippetExecutionConfig,
    markdown::elements::{Snippet, SnippetLanguage},
};
use once_cell::sync::Lazy;
use os_pipe::PipeReader;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead, BufReader, Write},
    process::{self, Child, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;

static EXECUTORS: Lazy<BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>> =
    Lazy::new(|| serde_yaml::from_slice(include_bytes!("../executors.yaml")).expect("executors.yaml is broken"));

/// Allows executing code.
#[derive(Debug)]
pub struct SnippetExecutor {
    executors: BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>,
}

impl SnippetExecutor {
    pub fn new(
        custom_executors: BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>,
    ) -> Result<Self, InvalidSnippetConfig> {
        let mut executors = EXECUTORS.clone();
        executors.extend(custom_executors);
        for (language, config) in &executors {
            if config.filename.is_empty() {
                return Err(InvalidSnippetConfig(language.clone(), "filename is empty"));
            }
            if config.commands.is_empty() {
                return Err(InvalidSnippetConfig(language.clone(), "no commands given"));
            }
            for command in &config.commands {
                if command.is_empty() {
                    return Err(InvalidSnippetConfig(language.clone(), "empty command given"));
                }
            }
        }
        Ok(Self { executors })
    }

    pub(crate) fn is_execution_supported(&self, language: &SnippetLanguage) -> bool {
        self.executors.contains_key(language)
    }

    /// Execute a piece of code.
    pub(crate) fn execute(&self, code: &Snippet) -> Result<ExecutionHandle, CodeExecuteError> {
        if !code.attributes.execute {
            return Err(CodeExecuteError::NotExecutableCode);
        }
        let Some(config) = self.executors.get(&code.language) else {
            return Err(CodeExecuteError::UnsupportedExecution);
        };
        Self::execute_lang(config, code.executable_contents().as_bytes())
    }

    fn execute_lang(config: &LanguageSnippetExecutionConfig, code: &[u8]) -> Result<ExecutionHandle, CodeExecuteError> {
        let script_dir =
            tempfile::Builder::default().prefix(".presenterm").tempdir().map_err(CodeExecuteError::TempDir)?;
        let snippet_path = script_dir.path().join(&config.filename);
        {
            let mut snippet_file = File::create(snippet_path).map_err(CodeExecuteError::TempDir)?;
            snippet_file.write_all(code).map_err(CodeExecuteError::TempDir)?;
        }

        let state: Arc<Mutex<ExecutionState>> = Default::default();
        let reader_handle =
            CommandsRunner::spawn(state.clone(), script_dir, config.commands.clone(), config.environment.clone());
        let handle = ExecutionHandle { state, reader_handle };
        Ok(handle)
    }
}

impl Default for SnippetExecutor {
    fn default() -> Self {
        Self::new(Default::default()).expect("initialization failed")
    }
}

/// An invalid executor was found.
#[derive(thiserror::Error, Debug)]
#[error("invalid snippet execution for '{0:?}': {1}")]
pub struct InvalidSnippetConfig(SnippetLanguage, &'static str);

/// An error during the execution of some code.
#[derive(thiserror::Error, Debug)]
pub(crate) enum CodeExecuteError {
    #[error("code language doesn't support execution")]
    UnsupportedExecution,

    #[error("code is not marked for execution")]
    NotExecutableCode,

    #[error("error creating temporary directory: {0}")]
    TempDir(io::Error),

    #[error("error spawning process '{0}': {1}")]
    SpawnProcess(String, io::Error),

    #[error("error creating pipe: {0}")]
    Pipe(io::Error),
}

/// A handle for the execution of a piece of code.
#[derive(Debug)]
pub(crate) struct ExecutionHandle {
    state: Arc<Mutex<ExecutionState>>,
    #[allow(dead_code)]
    reader_handle: thread::JoinHandle<()>,
}

impl ExecutionHandle {
    /// Get the current state of the process.
    pub(crate) fn state(&self) -> ExecutionState {
        self.state.lock().unwrap().clone()
    }
}

/// Consumes the output of a process and stores it in a shared state.
struct CommandsRunner {
    state: Arc<Mutex<ExecutionState>>,
    script_directory: TempDir,
}

impl CommandsRunner {
    fn spawn(
        state: Arc<Mutex<ExecutionState>>,
        script_directory: TempDir,
        commands: Vec<Vec<String>>,
        env: HashMap<String, String>,
    ) -> thread::JoinHandle<()> {
        let reader = Self { state, script_directory };
        thread::spawn(|| reader.run(commands, env))
    }

    fn run(self, commands: Vec<Vec<String>>, env: HashMap<String, String>) {
        let mut last_result = true;
        for command in commands {
            last_result = self.run_command(command, &env);
            if !last_result {
                break;
            }
        }
        let status = match last_result {
            true => ProcessStatus::Success,
            false => ProcessStatus::Failure,
        };
        self.state.lock().unwrap().status = status;
    }

    fn run_command(&self, command: Vec<String>, env: &HashMap<String, String>) -> bool {
        let (mut child, reader) = match self.launch_process(command, env) {
            Ok(inner) => inner,
            Err(e) => {
                let mut state = self.state.lock().unwrap();
                state.status = ProcessStatus::Failure;
                state.output.push(e.to_string());
                return false;
            }
        };
        let _ = Self::process_output(self.state.clone(), reader);

        match child.wait() {
            Ok(code) => code.success(),
            _ => false,
        }
    }

    fn launch_process(
        &self,
        commands: Vec<String>,
        env: &HashMap<String, String>,
    ) -> Result<(Child, PipeReader), CodeExecuteError> {
        let (reader, writer) = os_pipe::pipe().map_err(CodeExecuteError::Pipe)?;
        let writer_clone = writer.try_clone().map_err(CodeExecuteError::Pipe)?;
        let (command, args) = commands.split_first().expect("no commands");
        let child = process::Command::new(command)
            .args(args)
            .envs(env)
            .current_dir(self.script_directory.path())
            .stdin(Stdio::null())
            .stdout(writer)
            .stderr(writer_clone)
            .spawn()
            .map_err(|e| CodeExecuteError::SpawnProcess(command.clone(), e))?;
        Ok((child, reader))
    }

    fn process_output(state: Arc<Mutex<ExecutionState>>, reader: os_pipe::PipeReader) -> io::Result<()> {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let line = line?;
            // TODO: consider not locking per line...
            state.lock().unwrap().output.push(line);
        }
        Ok(())
    }
}

/// The state of the execution of a process.
#[derive(Clone, Default, Debug)]
pub(crate) struct ExecutionState {
    pub(crate) output: Vec<String>,
    pub(crate) status: ProcessStatus,
}

/// The status of a process.
#[derive(Clone, Debug, Default)]
pub(crate) enum ProcessStatus {
    #[default]
    Running,
    Success,
    Failure,
}

impl ProcessStatus {
    /// Check whether the underlying process is finished.
    pub(crate) fn is_finished(&self) -> bool {
        matches!(self, ProcessStatus::Success | ProcessStatus::Failure)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::markdown::elements::SnippetAttributes;

    #[test]
    fn shell_code_execution() {
        let contents = r"
echo 'hello world'
echo 'bye'"
            .into();
        let code = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes { execute: true, ..Default::default() },
        };
        let handle = SnippetExecutor::default().execute(&code).expect("execution failed");
        let state = loop {
            let state = handle.state();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected_lines = vec!["hello world", "bye"];
        assert_eq!(state.output, expected_lines);
    }

    #[test]
    fn non_executable_code_cant_be_executed() {
        let contents = String::new();
        let code = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes { execute: false, ..Default::default() },
        };
        let result = SnippetExecutor::default().execute(&code);
        assert!(result.is_err());
    }

    #[test]
    fn shell_code_execution_captures_stderr() {
        let contents = r"
echo 'This message redirects to stderr' >&2
echo 'hello world'
"
        .into();
        let code = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes { execute: true, ..Default::default() },
        };
        let handle = SnippetExecutor::default().execute(&code).expect("execution failed");
        let state = loop {
            let state = handle.state();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected_lines = vec!["This message redirects to stderr", "hello world"];
        assert_eq!(state.output, expected_lines);
    }

    #[test]
    fn shell_code_execution_executes_hidden_lines() {
        let contents = r"
/// echo 'this line was hidden'
/// echo 'this line was hidden and contains another prefix /// '
echo 'hello world'
"
        .into();
        let code = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes { execute: true, ..Default::default() },
        };
        let handle = SnippetExecutor::default().execute(&code).expect("execution failed");
        let state = loop {
            let state = handle.state();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected_lines =
            vec!["this line was hidden", "this line was hidden and contains another prefix /// ", "hello world"];
        assert_eq!(state.output, expected_lines);
    }

    #[test]
    fn built_in_executors() {
        SnippetExecutor::new(Default::default()).expect("invalid default executors");
    }
}
