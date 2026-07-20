use std::path::PathBuf;

use crate::{
    TerminalCommandRequest, TerminalExecutionResult, TerminalExecutionStatus, TerminalRiskClass,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalOutputEvent {
    terminal_id: String,
    workspace_id: String,
    command: String,
    working_directory: PathBuf,
    risk_class: TerminalRiskClass,
    status: TerminalExecutionStatus,
    stdout: String,
    stderr: String,
    redacted: bool,
}

impl TerminalOutputEvent {
    #[must_use]
    pub fn from_result(
        terminal_id: impl Into<String>,
        request: &TerminalCommandRequest,
        result: &TerminalExecutionResult,
    ) -> Self {
        let stdout = result.stdout().to_string();
        let stderr = result.stderr().to_string();
        let redacted = stdout.contains("[REDACTED]") || stderr.contains("[REDACTED]");
        Self {
            terminal_id: terminal_id.into(),
            workspace_id: request.workspace_id().to_string(),
            command: request.command().to_string(),
            working_directory: request.working_directory(),
            risk_class: request.risk_class(),
            status: result.status(),
            stdout,
            stderr,
            redacted,
        }
    }

    #[must_use]
    pub fn terminal_id(&self) -> &str {
        &self.terminal_id
    }

    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    #[must_use]
    pub fn redacted(&self) -> bool {
        self.redacted
    }
}
