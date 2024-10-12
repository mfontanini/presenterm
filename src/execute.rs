//! Code execution.

use crate::{
    custom::LanguageSnippetExecutionConfig,
    processing::code::{Snippet, SnippetLanguage},
};
use once_cell::sync::Lazy;
use os_pipe::PipeReader;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self, Debug},
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{self, Child, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;

static EXECUTORS: Lazy<BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>> =
    Lazy::new(|| serde_yaml::from_slice(include_bytes!("../executors.yaml")).expect("executors.yaml is broken"));

/// Allows executing code.
pub struct SnippetExecutor {
    executors: BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>,
    cwd: PathBuf,
}

impl SnippetExecutor {
    pub fn new(
        custom_executors: BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>,
        cwd: PathBuf,
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
        Ok(Self { executors, cwd })
    }

    pub(crate) fn is_execution_supported(&self, language: &SnippetLanguage) -> bool {
        self.executors.contains_key(language)
    }

    /// Execute a piece of code asynchronously.
    pub(crate) fn execute_async(&self, snippet: &Snippet) -> Result<ExecutionHandle, CodeExecuteError> {
        let config = self.language_config(snippet)?;
        let script_dir = Self::write_snippet(snippet, config)?;
        let state: Arc<Mutex<ExecutionState>> = Default::default();
        let reader_handle = CommandsRunner::spawn(
            state.clone(),
            script_dir,
            config.commands.clone(),
            config.environment.clone(),
            self.cwd.to_path_buf(),
        );
        let handle = ExecutionHandle { state, reader_handle };
        Ok(handle)
    }

    /// Executes a piece of code synchronously.
    pub(crate) fn execute_sync(&self, snippet: &Snippet) -> Result<(), CodeExecuteError> {
        let config = self.language_config(snippet)?;
        let script_dir = Self::write_snippet(snippet, config)?;
        let script_dir_path = script_dir.path().to_string_lossy();
        for mut commands in config.commands.clone() {
            for command in &mut commands {
                *command = command.replace("$pwd", &script_dir_path);
            }
            let (command, args) = commands.split_first().expect("no commands");
            let child = process::Command::new(command)
                .args(args)
                .envs(&config.environment)
                .current_dir(&self.cwd)
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| CodeExecuteError::SpawnProcess(command.clone(), e))?;

            let output = child.wait_with_output().map_err(CodeExecuteError::Waiting)?;
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(CodeExecuteError::Running(error));
            }
        }
        Ok(())
    }

    pub(crate) fn hidden_line_prefix(&self, language: &SnippetLanguage) -> Option<&str> {
        self.executors.get(language).and_then(|lang| lang.hidden_line_prefix.as_deref())
    }

    fn language_config(&self, snippet: &Snippet) -> Result<&LanguageSnippetExecutionConfig, CodeExecuteError> {
        if !snippet.attributes.execute && !snippet.attributes.execute_replace {
            return Err(CodeExecuteError::NotExecutableCode);
        }
        self.executors.get(&snippet.language).ok_or(CodeExecuteError::UnsupportedExecution)
    }

    fn write_snippet(snippet: &Snippet, config: &LanguageSnippetExecutionConfig) -> Result<TempDir, CodeExecuteError> {
        let hide_prefix = config.hidden_line_prefix.as_deref();
        let code = snippet.executable_contents(hide_prefix);
        let script_dir =
            tempfile::Builder::default().prefix(".presenterm").tempdir().map_err(CodeExecuteError::TempDir)?;
        let snippet_path = script_dir.path().join(&config.filename);
        let mut snippet_file = File::create(snippet_path).map_err(CodeExecuteError::TempDir)?;
        snippet_file.write_all(code.as_bytes()).map_err(CodeExecuteError::TempDir)?;
        Ok(script_dir)
    }
}

impl Default for SnippetExecutor {
    fn default() -> Self {
        Self::new(Default::default(), PathBuf::from("./")).expect("initialization failed")
    }
}

impl Debug for SnippetExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SnippetExecutor {{ .. }}")
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

    #[error("error waiting for process to run: {0}")]
    Waiting(io::Error),

    #[error("error running process: {0}")]
    Running(String),
}

/// A handle for the execution of a piece of code.
#[derive(Debug)]
pub(crate) struct ExecutionHandle {
    pub(crate) state: Arc<Mutex<ExecutionState>>,
    #[allow(dead_code)]
    reader_handle: thread::JoinHandle<()>,
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
        cwd: PathBuf,
    ) -> thread::JoinHandle<()> {
        let reader = Self { state, script_directory };
        thread::spawn(|| reader.run(commands, env, cwd))
    }

    fn run(self, commands: Vec<Vec<String>>, env: HashMap<String, String>, cwd: PathBuf) {
        let mut last_result = true;
        for command in commands {
            last_result = self.run_command(command, &env, &cwd);
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

    fn run_command(&self, command: Vec<String>, env: &HashMap<String, String>, cwd: &Path) -> bool {
        let (mut child, reader) = match self.launch_process(command, env, cwd) {
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
        mut commands: Vec<String>,
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<(Child, PipeReader), CodeExecuteError> {
        let (reader, writer) = os_pipe::pipe().map_err(CodeExecuteError::Pipe)?;
        let writer_clone = writer.try_clone().map_err(CodeExecuteError::Pipe)?;
        let script_dir = self.script_directory.path().to_string_lossy();
        for command in &mut commands {
            *command = command.replace("$pwd", &script_dir);
        }
        let (command, args) = commands.split_first().expect("no commands");
        let child = process::Command::new(command)
            .args(args)
            .envs(env)
            .current_dir(cwd)
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
            let mut line = line?;
            if line.contains('\t') {
                line = line.replace('\t', "    ");
            }
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
    use crate::processing::code::SnippetAttributes;

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
        let handle = SnippetExecutor::default().execute_async(&code).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
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
        let result = SnippetExecutor::default().execute_async(&code);
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
        let handle = SnippetExecutor::default().execute_async(&code).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
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
        let handle = SnippetExecutor::default().execute_async(&code).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
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
        SnippetExecutor::new(Default::default(), PathBuf::from("./")).expect("invalid default executors");
    }
}
