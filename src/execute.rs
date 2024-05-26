//! Code execution.

use crate::markdown::elements::{Code, CodeLanguage};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{self, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tempfile::NamedTempFile;

include!(concat!(env!("OUT_DIR"), "/executors.rs"));

/// Allows executing code.
#[derive(Default, Debug)]
pub struct CodeExecutor {
    custom_executors: BTreeMap<CodeLanguage, Vec<u8>>,
}

impl CodeExecutor {
    pub fn load(executors_path: &Path) -> Result<Self, LoadExecutorsError> {
        let mut custom_executors = BTreeMap::new();
        if let Ok(paths) = fs::read_dir(executors_path) {
            for executor in paths {
                let executor = executor?;
                let path = executor.path();
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                let Some((name, extension)) = filename.split_once('.') else {
                    return Err(LoadExecutorsError::InvalidExecutor(path, "no extension"));
                };
                if extension != "sh" {
                    return Err(LoadExecutorsError::InvalidExecutor(path, "non .sh extension"));
                }
                let language: CodeLanguage = match name.parse() {
                    Ok(language) => language,
                    Err(_) => return Err(LoadExecutorsError::InvalidExecutor(path, "invalid code language")),
                };
                let file_contents = fs::read(path)?;
                custom_executors.insert(language, file_contents);
            }
        }
        Ok(Self { custom_executors })
    }

    pub(crate) fn is_execution_supported(&self, language: &CodeLanguage) -> bool {
        if matches!(language, CodeLanguage::Shell(_)) {
            true
        } else {
            EXECUTORS.contains_key(language) || self.custom_executors.contains_key(language)
        }
    }

    /// Execute a piece of code.
    pub(crate) fn execute(&self, code: &Code) -> Result<ExecutionHandle, CodeExecuteError> {
        if !code.attributes.execute {
            return Err(CodeExecuteError::NotExecutableCode);
        }
        match &code.language {
            CodeLanguage::Shell(interpreter) => {
                let args: &[&str] = &[];
                Self::execute_shell(interpreter, code.executable_contents().as_bytes(), args)
            }
            lang => {
                let executor = self.executor(lang).ok_or(CodeExecuteError::UnsupportedExecution)?;
                Self::execute_lang(executor, code.executable_contents().as_bytes())
            }
        }
    }

    fn executor(&self, language: &CodeLanguage) -> Option<&[u8]> {
        if let Some(executor) = self.custom_executors.get(language) {
            return Some(executor);
        }
        EXECUTORS.get(language).copied()
    }

    fn execute_shell<S>(interpreter: &str, code: &[u8], args: &[S]) -> Result<ExecutionHandle, CodeExecuteError>
    where
        S: AsRef<OsStr>,
    {
        let mut output_file = NamedTempFile::new().map_err(CodeExecuteError::TempFile)?;
        output_file.write_all(code).map_err(CodeExecuteError::TempFile)?;
        output_file.flush().map_err(CodeExecuteError::TempFile)?;
        let (reader, writer) = os_pipe::pipe().map_err(CodeExecuteError::Pipe)?;
        let writer_clone = writer.try_clone().map_err(CodeExecuteError::Pipe)?;
        let process_handle = process::Command::new("/usr/bin/env")
            .arg(interpreter)
            .arg(output_file.path())
            .args(args)
            .stdin(Stdio::null())
            .stdout(writer)
            .stderr(writer_clone)
            .spawn()
            .map_err(CodeExecuteError::SpawnProcess)?;

        let state: Arc<Mutex<ExecutionState>> = Default::default();
        let reader_handle = ProcessReader::spawn(process_handle, state.clone(), output_file, reader);
        let handle = ExecutionHandle { state, reader_handle, program_path: None };
        Ok(handle)
    }

    fn execute_lang(executor: &[u8], code: &[u8]) -> Result<ExecutionHandle, CodeExecuteError> {
        let mut code_file = NamedTempFile::new().map_err(CodeExecuteError::TempFile)?;
        code_file.write_all(code).map_err(CodeExecuteError::TempFile)?;

        let path = code_file.path();
        let mut handle = Self::execute_shell("bash", executor, &[path])?;
        handle.program_path = Some(code_file);
        Ok(handle)
    }
}

/// An error during the load of custom executors.
#[derive(thiserror::Error, Debug)]
pub enum LoadExecutorsError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid executor '{0}': {1}")]
    InvalidExecutor(PathBuf, &'static str),
}

/// An error during the execution of some code.
#[derive(thiserror::Error, Debug)]
pub(crate) enum CodeExecuteError {
    #[error("code language doesn't support execution")]
    UnsupportedExecution,

    #[error("code is not marked for execution")]
    NotExecutableCode,

    #[error("error creating temporary file: {0}")]
    TempFile(io::Error),

    #[error("error spawning process: {0}")]
    SpawnProcess(io::Error),

    #[error("error creating pipe: {0}")]
    Pipe(io::Error),
}

/// A handle for the execution of a piece of code.
#[derive(Debug)]
pub(crate) struct ExecutionHandle {
    state: Arc<Mutex<ExecutionState>>,
    #[allow(dead_code)]
    reader_handle: thread::JoinHandle<()>,
    program_path: Option<NamedTempFile>,
}

impl ExecutionHandle {
    /// Get the current state of the process.
    pub(crate) fn state(&self) -> ExecutionState {
        self.state.lock().unwrap().clone()
    }
}

/// Consumes the output of a process and stores it in a shared state.
struct ProcessReader {
    handle: process::Child,
    state: Arc<Mutex<ExecutionState>>,
    #[allow(dead_code)]
    file_handle: NamedTempFile,
    reader: os_pipe::PipeReader,
}

impl ProcessReader {
    fn spawn(
        handle: process::Child,
        state: Arc<Mutex<ExecutionState>>,
        file_handle: NamedTempFile,
        reader: os_pipe::PipeReader,
    ) -> thread::JoinHandle<()> {
        let reader = Self { handle, state, file_handle, reader };
        thread::spawn(|| reader.run())
    }

    fn run(mut self) {
        let _ = Self::process_output(self.state.clone(), self.reader);
        let success = match self.handle.wait() {
            Ok(code) => code.success(),
            _ => false,
        };
        let status = match success {
            true => ProcessStatus::Success,
            false => ProcessStatus::Failure,
        };
        self.state.lock().unwrap().status = status;
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
    use crate::markdown::elements::CodeAttributes;

    #[test]
    fn shell_code_execution() {
        let contents = r"
echo 'hello world'
echo 'bye'"
            .into();
        let code = Code {
            contents,
            language: CodeLanguage::Shell("sh".into()),
            attributes: CodeAttributes { execute: true, ..Default::default() },
        };
        let handle = CodeExecutor::default().execute(&code).expect("execution failed");
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
        let code = Code {
            contents,
            language: CodeLanguage::Shell("sh".into()),
            attributes: CodeAttributes { execute: false, ..Default::default() },
        };
        let result = CodeExecutor::default().execute(&code);
        assert!(result.is_err());
    }

    #[test]
    fn shell_code_execution_captures_stderr() {
        let contents = r"
echo 'This message redirects to stderr' >&2
echo 'hello world'
"
        .into();
        let code = Code {
            contents,
            language: CodeLanguage::Shell("sh".into()),
            attributes: CodeAttributes { execute: true, ..Default::default() },
        };
        let handle = CodeExecutor::default().execute(&code).expect("execution failed");
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
/// echo 'this line was hidden and contains another delimiter /// '
echo 'hello world'
"
        .into();
        let code = Code {
            contents,
            language: CodeLanguage::Shell("sh".into()),
            attributes: CodeAttributes { execute: true, ..Default::default() },
        };
        let handle = CodeExecutor::default().execute(&code).expect("execution failed");
        let state = loop {
            let state = handle.state();
            if state.status.is_finished() {
                break state;
            }
        };

        let expected_lines =
            vec!["this line was hidden", "this line was hidden and contains another delimiter /// ", "hello world"];
        assert_eq!(state.output, expected_lines);
    }
}
