use std::path::{Path, PathBuf};
use std::time::Duration;

use desktoplab_policy::{Action, PolicyEngine};
use desktoplab_redaction::redact_sensitive;

use crate::{
    TerminalProcessAdapter, TerminalProcessRequest, TerminalProcessStatus, TerminalRiskClass,
    ToolGateway, ToolIntent, ToolOutcome,
    path_security::{contained_existing_path, relative_workspace_path},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalApproval {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCommandRequest {
    workspace_id: String,
    command: String,
    working_directory: PathBuf,
    risk_class: TerminalRiskClass,
    approval_state: TerminalApproval,
}

impl TerminalCommandRequest {
    #[must_use]
    pub fn new(workspace_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            command: command.into(),
            working_directory: PathBuf::new(),
            risk_class: TerminalRiskClass::Medium,
            approval_state: TerminalApproval::Pending,
        }
    }

    #[must_use]
    pub fn for_workspace(workspace_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self::new(workspace_id, command)
    }

    #[must_use]
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    #[must_use]
    pub fn with_risk_class(mut self, risk_class: TerminalRiskClass) -> Self {
        self.risk_class = risk_class;
        self
    }

    #[must_use]
    pub fn with_approval_state(mut self, approval_state: TerminalApproval) -> Self {
        self.approval_state = approval_state;
        self
    }

    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    #[must_use]
    pub fn working_directory(&self) -> PathBuf {
        self.working_directory.clone()
    }

    #[must_use]
    pub fn risk_class(&self) -> TerminalRiskClass {
        self.risk_class
    }

    #[must_use]
    pub fn approval_state(&self) -> TerminalApproval {
        self.approval_state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalExecutionStatus {
    Exited(i32),
    TimedOut,
    FailedToSpawn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalExecutionResult {
    status: TerminalExecutionStatus,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

impl TerminalExecutionResult {
    #[must_use]
    pub fn status(&self) -> TerminalExecutionStatus {
        self.status.clone()
    }

    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    #[must_use]
    pub fn stdout_truncated(&self) -> bool {
        self.stdout_truncated
    }

    #[must_use]
    pub fn stderr_truncated(&self) -> bool {
        self.stderr_truncated
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalToolOutcome {
    Completed(TerminalExecutionResult),
    ApprovalRequired,
    Denied,
    Blocked(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalToolExecutor {
    root: PathBuf,
    gateway: ToolGateway,
    timeout: Duration,
    output_limit: usize,
}

impl TerminalToolExecutor {
    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine, timeout: Duration, output_limit: usize) -> Self {
        Self {
            root: root.to_path_buf(),
            gateway: ToolGateway::new(policy),
            timeout,
            output_limit,
        }
    }

    pub fn execute(
        &mut self,
        request: TerminalCommandRequest,
        approval: TerminalApproval,
    ) -> TerminalToolOutcome {
        let Ok(cwd) = self.resolve_workspace_path(&request.working_directory) else {
            return TerminalToolOutcome::Blocked("path_escape");
        };

        match self.gateway.authorize(ToolIntent::terminal_workspace(
            &request.workspace_id,
            request.working_directory.to_string_lossy(),
            &request.command,
            request.risk_class,
        )) {
            ToolOutcome::Blocked(_) => return TerminalToolOutcome::Blocked("policy_denied"),
            ToolOutcome::Allowed(Action::TerminalCommand | Action::DependencyInstall) => {}
            ToolOutcome::ApprovalRequired(Action::TerminalCommand | Action::DependencyInstall) => {
                match approval {
                    TerminalApproval::Pending => return TerminalToolOutcome::ApprovalRequired,
                    TerminalApproval::Denied => return TerminalToolOutcome::Denied,
                    TerminalApproval::Approved => {}
                }
            }
            ToolOutcome::Allowed(_) | ToolOutcome::ApprovalRequired(_) => {
                return TerminalToolOutcome::Blocked("unexpected_action");
            }
        }

        TerminalToolOutcome::Completed(self.spawn_command(&cwd, &request.command))
    }

    #[must_use]
    pub fn approval_count(&self) -> usize {
        self.gateway.approval_requests().len()
    }

    #[must_use]
    pub fn audit_count(&self) -> usize {
        self.gateway.audit_records().len()
    }

    fn spawn_command(&self, cwd: &Path, command: &str) -> TerminalExecutionResult {
        let output = TerminalProcessAdapter::new(self.timeout, self.output_limit)
            .run(TerminalProcessRequest::new(command, cwd));
        let status = match output.status() {
            TerminalProcessStatus::Exited(code) => TerminalExecutionStatus::Exited(code),
            TerminalProcessStatus::TimedOut => TerminalExecutionStatus::TimedOut,
            TerminalProcessStatus::FailedToSpawn => TerminalExecutionStatus::FailedToSpawn,
        };
        let (stdout, stdout_redaction_truncated) =
            bounded_redacted(output.stdout(), self.output_limit);
        let (stderr, stderr_redaction_truncated) =
            bounded_redacted(output.stderr(), self.output_limit);
        TerminalExecutionResult {
            status,
            stdout,
            stderr,
            stdout_truncated: output.stdout_truncated() || stdout_redaction_truncated,
            stderr_truncated: output.stderr_truncated() || stderr_redaction_truncated,
        }
    }

    fn resolve_workspace_path(&self, path: &Path) -> Result<PathBuf, &'static str> {
        let candidate = relative_workspace_path(&self.root, path).map_err(|_| "path_escape")?;
        contained_existing_path(&self.root, &candidate).map_err(|_| "path_escape")
    }
}

fn bounded_redacted(value: &str, limit: usize) -> (String, bool) {
    let text = redact_sensitive(value);
    if text.len() <= limit {
        return (text, false);
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::bounded_redacted;

    #[test]
    fn redacted_output_limit_is_measured_in_bytes() {
        let (bounded, truncated) = bounded_redacted("ééé", 5);

        assert!(bounded.len() <= 5);
        assert!(truncated);
    }
}
