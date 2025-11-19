//! Code execution.

use super::snippet::SnippetExecutorSpec;
use crate::{
    code::snippet::{Snippet, SnippetExecution, SnippetLanguage, SnippetRepr},
    config::{LanguageSnippetExecutionConfig, SnippetExecutorConfig},
};
use once_cell::sync::Lazy;
use os_pipe::PipeReader;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self, Debug},
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{self, Child, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;

static EXECUTORS: Lazy<BTreeMap<SnippetLanguage, LanguageSnippetExecutionConfig>> =
    Lazy::new(|| serde_yaml::from_slice(include_bytes!("../../executors.yaml")).expect("executors.yaml is broken"));

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
            Self::validate_executor_config(language, &config.executor)?;
            for alternative in config.alternative.values() {
                Self::validate_executor_config(language, alternative)?;
            }
        }
        Ok(Self { executors, cwd })
    }

    pub(crate) fn language_executor(
        &self,
        language: &SnippetLanguage,
        spec: &SnippetExecutorSpec,
    ) -> Result<LanguageSnippetExecutor, UnsupportedExecution> {
        let language_config = self
            .executors
            .get(language)
            .ok_or_else(|| UnsupportedExecution(language.clone(), "no executors found".into()))?;
        let config = match spec {
            SnippetExecutorSpec::Default => language_config.executor.clone(),
            SnippetExecutorSpec::Alternative(name) => {
                language_config.alternative.get(name).cloned().ok_or_else(|| {
                    UnsupportedExecution(language.clone(), format!("alternative executor '{name}' is not defined"))
                })?
            }
        };
        Ok(LanguageSnippetExecutor {
            hidden_line_prefix: language_config.hidden_line_prefix.clone(),
            config,
            cwd: self.cwd.clone(),
        })
    }

    pub(crate) fn hidden_line_prefix(&self, language: &SnippetLanguage) -> Option<&str> {
        self.executors.get(language).and_then(|lang| lang.hidden_line_prefix.as_deref())
    }

    fn validate_executor_config(
        language: &SnippetLanguage,
        executor: &SnippetExecutorConfig,
    ) -> Result<(), InvalidSnippetConfig> {
        if executor.filename.is_empty() {
            return Err(InvalidSnippetConfig(language.clone(), "filename is empty"));
        }
        if executor.commands.is_empty() {
            return Err(InvalidSnippetConfig(language.clone(), "no commands given"));
        }
        for command in &executor.commands {
            if command.is_empty() {
                return Err(InvalidSnippetConfig(language.clone(), "empty command given"));
            }
        }
        Ok(())
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

#[derive(Clone, Debug)]
pub(crate) struct LanguageSnippetExecutor {
    hidden_line_prefix: Option<String>,
    config: SnippetExecutorConfig,
    cwd: PathBuf,
}

impl LanguageSnippetExecutor {
    /// Execute a piece of code asynchronously.
    pub(crate) fn execute_async(&self, snippet: &Snippet) -> Result<ExecutionHandle, CodeExecuteError> {
        let script_dir = self.write_snippet(snippet)?;
        let state: Arc<Mutex<ExecutionState>> = Default::default();
        let output_type = match &snippet.attributes.execution {
            SnippetExecution::Exec(args) if matches!(args.repr, SnippetRepr::Image) => OutputType::Binary,
            _ => OutputType::Lines,
        };
        let reader_handle = CommandsRunner::spawn(
            state.clone(),
            script_dir,
            self.config.commands.clone(),
            self.config.environment.clone(),
            self.cwd.clone(),
            output_type,
        );
        let handle = ExecutionHandle { state, reader_handle };
        Ok(handle)
    }

    /// Executes a piece of code synchronously.
    pub(crate) fn execute_sync(&self, snippet: &Snippet) -> Result<(), CodeExecuteError> {
        let script_dir = self.write_snippet(snippet)?;
        let script_dir_path = script_dir.path().to_string_lossy();
        for commands in self.config.commands.clone() {
            self.execute_command(commands, &script_dir_path)?;
        }
        Ok(())
    }

    /// Creates the necessary context to run this snippet in a PTY.
    pub(crate) fn pty_execution_context(&self, snippet: &Snippet) -> Result<PtySnippetContext, CodeExecuteError> {
        let script_dir = self.write_snippet(snippet)?;
        let script_dir_path = script_dir.path().to_string_lossy();

        // Run the first N-1 commands normally and assume the last one is the one that actually
        // invokes the thing (e.g. rust snippet compilation happens here, snippet execution in PTY)
        for commands in self.config.commands.iter().take(self.config.commands.len() - 1).cloned() {
            self.execute_command(commands, &script_dir_path)?;
        }
        let mut commands = self.config.commands.last().cloned().unwrap();
        for command in &mut commands {
            *command = command.replace("$pwd", &script_dir_path);
        }
        let (command, args) = commands.split_first().expect("no commands");
        let mut command = portable_pty::CommandBuilder::new(command);
        command.args(args);
        command.cwd(&self.cwd);
        for (key, value) in &self.config.environment {
            command.env(key, value);
        }
        Ok(PtySnippetContext { command, _temp: script_dir })
    }

    fn execute_command(&self, mut commands: Vec<String>, script_dir_path: &str) -> Result<(), CodeExecuteError> {
        for command in &mut commands {
            *command = command.replace("$pwd", script_dir_path);
        }
        let (command, args) = commands.split_first().expect("no commands");
        let child = process::Command::new(command)
            .args(args)
            .envs(&self.config.environment)
            .current_dir(&self.cwd)
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CodeExecuteError::SpawnProcess(command.clone(), e))?;

        let output = child.wait_with_output().map_err(CodeExecuteError::Waiting)?;
        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(CodeExecuteError::Running(error))
        }
    }

    fn write_snippet(&self, snippet: &Snippet) -> Result<TempDir, CodeExecuteError> {
        let hide_prefix = self.hidden_line_prefix.as_deref();
        let code = snippet.executable_contents(hide_prefix);
        let script_dir =
            tempfile::Builder::default().prefix(".presenterm").tempdir().map_err(CodeExecuteError::TempDir)?;
        let snippet_path = script_dir.path().join(&self.config.filename);
        let mut snippet_file = File::create(snippet_path).map_err(CodeExecuteError::TempDir)?;
        snippet_file.write_all(code.as_bytes()).map_err(CodeExecuteError::TempDir)?;
        Ok(script_dir)
    }
}

pub(crate) struct PtySnippetContext {
    pub(crate) command: portable_pty::CommandBuilder,
    _temp: TempDir,
}

/// An invalid executor was found.
#[derive(thiserror::Error, Debug)]
#[error("invalid snippet execution for '{0:?}': {1}")]
pub struct InvalidSnippetConfig(SnippetLanguage, &'static str);

/// Execution for a language is unsupported.
#[derive(thiserror::Error, Debug)]
#[error("cannot execute code for '{0:?}': {1}")]
pub struct UnsupportedExecution(SnippetLanguage, String);

/// An error during the execution of some code.
#[derive(thiserror::Error, Debug)]
pub(crate) enum CodeExecuteError {
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
        output_type: OutputType,
    ) -> thread::JoinHandle<()> {
        let reader = Self { state, script_directory };
        thread::spawn(move || reader.run(commands, env, cwd, output_type))
    }

    fn run(self, commands: Vec<Vec<String>>, env: HashMap<String, String>, cwd: PathBuf, output_type: OutputType) {
        let mut last_result = true;
        for command in commands {
            last_result = self.run_command(command, &env, &cwd, output_type);
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

    fn run_command(
        &self,
        command: Vec<String>,
        env: &HashMap<String, String>,
        cwd: &Path,
        output_type: OutputType,
    ) -> bool {
        let (mut child, reader) = match self.launch_process(command, env, cwd) {
            Ok(inner) => inner,
            Err(e) => {
                let mut state = self.state.lock().unwrap();
                state.status = ProcessStatus::Failure;
                state.output.extend(e.to_string().into_bytes());
                return false;
            }
        };
        let _ = Self::process_output(self.state.clone(), reader, output_type);

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

    fn process_output(
        state: Arc<Mutex<ExecutionState>>,
        mut reader: os_pipe::PipeReader,
        output_type: OutputType,
    ) -> io::Result<()> {
        match output_type {
            OutputType::Lines => {
                let reader = BufReader::new(reader);
                for line in reader.lines() {
                    let mut state = state.lock().unwrap();
                    state.output.extend(line?.into_bytes());
                    state.output.push(b'\n');
                }
                Ok(())
            }
            OutputType::Binary => {
                let mut buffer = Vec::new();
                reader.read_to_end(&mut buffer)?;
                state.lock().unwrap().output.extend(buffer);
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy)]
enum OutputType {
    Lines,
    Binary,
}

/// The state of the execution of a process.
#[derive(Clone, Default, Debug)]
pub(crate) struct ExecutionState {
    pub(crate) output: Vec<u8>,
    pub(crate) status: ProcessStatus,
}

/// The status of a process.
#[derive(Clone, Copy, Debug, Default)]
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
    use crate::code::snippet::{SnippetAttributes, SnippetExecution};

    #[test]
    fn shell_code_execution() {
        let contents = r"
echo 'hello world'
echo 'bye'"
            .into();
        let snippet = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes {
                execution: SnippetExecution::Exec(Default::default()),
                ..Default::default()
            },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let handle = executor.execute_async(&snippet).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected = b"hello world\nbye\n";
        assert_eq!(state.output, expected);
    }

    #[test]
    fn shell_code_execution_captures_stderr() {
        let contents = r"
echo 'This message redirects to stderr' >&2
echo 'hello world'
"
        .into();
        let snippet = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes {
                execution: SnippetExecution::Exec(Default::default()),
                ..Default::default()
            },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let handle = executor.execute_async(&snippet).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected = b"This message redirects to stderr\nhello world\n";
        assert_eq!(state.output, expected);
    }

    #[test]
    fn shell_code_execution_executes_hidden_lines() {
        let contents = r"
/// echo 'this line was hidden'
/// echo 'this line was hidden and contains another prefix /// '
echo 'hello world'
"
        .into();
        let snippet = Snippet {
            contents,
            language: SnippetLanguage::Shell,
            attributes: SnippetAttributes {
                execution: SnippetExecution::Exec(Default::default()),
                ..Default::default()
            },
        };
        let executor = SnippetExecutor::default().language_executor(&snippet.language, &Default::default()).unwrap();
        let handle = executor.execute_async(&snippet).expect("execution failed");
        let state = loop {
            let state = handle.state.lock().unwrap();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected = b"this line was hidden\nthis line was hidden and contains another prefix /// \nhello world\n";
        assert_eq!(state.output, expected);
    }

    #[test]
    fn built_in_executors() {
        SnippetExecutor::new(Default::default(), PathBuf::from("./")).expect("invalid default executors");
    }
}
