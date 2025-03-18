use itertools::Itertools;
use std::{
    io::{self, Write},
    process::{Command, Output, Stdio},
};

const DEFAULT_MAX_ERROR_LINES: usize = 10;

pub(crate) struct ThirdPartyTools;

impl ThirdPartyTools {
    pub(crate) fn pandoc(args: &[&str]) -> Tool {
        Tool::new("pandoc", args)
    }

    pub(crate) fn typst(args: &[&str]) -> Tool {
        Tool::new("typst", args)
    }

    pub(crate) fn mermaid(args: &[&str]) -> Tool {
        let mmdc = if cfg!(windows) { "mmdc.cmd" } else { "mmdc" };
        Tool::new(mmdc, args)
    }

    pub(crate) fn weasyprint(args: &[&str]) -> Tool {
        Tool::new("weasyprint", args).inherit_stdout().max_error_lines(100)
    }
}

pub(crate) struct Tool {
    command_name: &'static str,
    command: Command,
    stdin: Option<Vec<u8>>,
    max_error_lines: usize,
}

impl Tool {
    fn new(command_name: &'static str, args: &[&str]) -> Self {
        let mut command = Command::new(command_name);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        Self { command_name, command, stdin: None, max_error_lines: DEFAULT_MAX_ERROR_LINES }
    }

    pub(crate) fn stdin(mut self, stdin: Vec<u8>) -> Self {
        self.stdin = Some(stdin);
        self
    }

    pub(crate) fn inherit_stdout(mut self) -> Self {
        self.command.stdout(Stdio::inherit());
        self
    }

    pub(crate) fn max_error_lines(mut self, value: usize) -> Self {
        self.max_error_lines = value;
        self
    }

    pub(crate) fn run(self) -> Result<(), ExecutionError> {
        self.spawn()?;
        Ok(())
    }

    pub(crate) fn run_and_capture_stdout(mut self) -> Result<Vec<u8>, ExecutionError> {
        self.command.stdout(Stdio::piped());

        let output = self.spawn()?;
        Ok(output.stdout)
    }

    fn spawn(mut self) -> Result<Output, ExecutionError> {
        use ExecutionError::*;
        if self.stdin.is_some() {
            self.command.stdin(Stdio::piped());
        }
        let mut child = match self.command.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(SpawnNotFound { command: self.command_name }),
            Err(error) => return Err(Spawn { command: self.command_name, error }),
        };
        if let Some(data) = &self.stdin {
            let mut stdin = child.stdin.take().expect("no stdin");
            stdin
                .write_all(data)
                .and_then(|_| stdin.flush())
                .map_err(|error| Communication { command: self.command_name, error })?;
        }
        let output = child.wait_with_output().map_err(|error| Communication { command: self.command_name, error })?;
        self.validate_output(&output)?;
        Ok(output)
    }

    fn validate_output(self, output: &Output) -> Result<(), ExecutionError> {
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).lines().take(self.max_error_lines).join("\n");
            Err(ExecutionError::Execution { command: self.command_name, stderr })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("spawning '{command}' failed: {error}")]
    Spawn { command: &'static str, error: io::Error },

    #[error("spawning '{command}' failed (is '{command}' installed?)")]
    SpawnNotFound { command: &'static str },

    #[error("communicating with '{command}' failed: {error}")]
    Communication { command: &'static str, error: io::Error },

    #[error("'{command}' execution failed: \n{stderr}")]
    Execution { command: &'static str, stderr: String },
}
