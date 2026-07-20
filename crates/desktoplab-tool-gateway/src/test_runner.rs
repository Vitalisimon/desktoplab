use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use desktoplab_policy::PolicyEngine;
use desktoplab_workspace::{DetectedTestCommand, TestCommandConfidence, TestCommandDetector};

use crate::{
    TerminalApproval, TerminalCommandRequest, TerminalExecutionStatus, TerminalRiskClass,
    TerminalToolExecutor, TerminalToolOutcome,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestRunApproval {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestRunRequest {
    workspace_id: String,
    command: String,
    reason: String,
    working_directory: PathBuf,
}

impl TestRunRequest {
    #[must_use]
    pub fn new(
        workspace_id: impl Into<String>,
        command: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            command: command.into(),
            reason: reason.into(),
            working_directory: PathBuf::new(),
        }
    }

    #[must_use]
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = path.into();
        self
    }

    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestRunEvidence {
    command: String,
    reason: String,
    status: TerminalExecutionStatus,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    duration_ms: u128,
    redaction_status: &'static str,
}

impl TestRunEvidence {
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

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
    pub fn duration_ms(&self) -> u128 {
        self.duration_ms
    }

    #[must_use]
    pub fn redaction_status(&self) -> &'static str {
        self.redaction_status
    }

    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Test command `{}` finished with status {:?} in {}ms. redaction_status={} stdout_truncated={} stderr_truncated={}\nstdout:\n{}\nstderr:\n{}",
            self.command,
            self.status,
            self.duration_ms,
            self.redaction_status,
            self.stdout_truncated,
            self.stderr_truncated,
            self.stdout,
            self.stderr
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestRunOutcome {
    Completed(TestRunEvidence),
    ApprovalRequired,
    Denied,
    Blocked(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedTestCommand {
    command: String,
    reason: String,
    confidence: TestCommandConfidence,
}

impl SelectedTestCommand {
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    #[must_use]
    pub fn confidence(&self) -> TestCommandConfidence {
        self.confidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestCommandSelection {
    Selected(SelectedTestCommand),
    ClarificationRequired {
        candidates: Vec<String>,
        reason: String,
    },
    Blocked(&'static str),
}

pub struct TestRunnerExecutor {
    root: PathBuf,
    policy: PolicyEngine,
    timeout: Duration,
    output_limit: usize,
}

impl TestRunnerExecutor {
    #[must_use]
    pub fn for_selection(root: &Path) -> Self {
        Self::new(
            root,
            PolicyEngine::default_conservative(),
            Duration::from_secs(30),
            4096,
        )
    }

    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine, timeout: Duration, output_limit: usize) -> Self {
        Self {
            root: root.to_path_buf(),
            policy,
            timeout,
            output_limit,
        }
    }

    pub fn detected_commands(&self) -> Result<Vec<String>, std::io::Error> {
        Ok(TestCommandDetector::detect(&self.root)?
            .commands()
            .iter()
            .map(|command| command.command().to_string())
            .collect())
    }

    pub fn select_project_command(&self) -> Result<TestCommandSelection, std::io::Error> {
        let detected = TestCommandDetector::detect(&self.root)?;
        let commands = detected.commands();
        if commands.is_empty() {
            return Ok(TestCommandSelection::Blocked("test_command_not_detected"));
        }
        let high = commands
            .iter()
            .filter(|command| command.confidence() == TestCommandConfidence::High)
            .collect::<Vec<_>>();
        if high.len() > 1 {
            return Ok(clarify(
                high,
                "multiple high-confidence test commands detected",
            ));
        }
        if high.len() == 1 {
            return Ok(select(high[0]));
        }
        if commands.len() == 1 {
            return Ok(select(&commands[0]));
        }
        Ok(clarify(
            commands.iter().collect(),
            "multiple possible test commands detected",
        ))
    }

    pub fn run(&mut self, request: TestRunRequest, approval: TestRunApproval) -> TestRunOutcome {
        if request.command.trim().is_empty() {
            return TestRunOutcome::Blocked("missing_test_command");
        }
        let approval = match approval {
            TestRunApproval::Pending => TerminalApproval::Pending,
            TestRunApproval::Approved => TerminalApproval::Approved,
            TestRunApproval::Denied => TerminalApproval::Denied,
        };
        let terminal_request =
            TerminalCommandRequest::for_workspace(&request.workspace_id, &request.command)
                .with_working_directory(&request.working_directory)
                .with_risk_class(TerminalRiskClass::Medium);
        let mut terminal = TerminalToolExecutor::new(
            &self.root,
            self.policy.clone(),
            self.timeout,
            self.output_limit,
        );
        let start = Instant::now();
        match terminal.execute(terminal_request, approval) {
            TerminalToolOutcome::Completed(result) => TestRunOutcome::Completed(TestRunEvidence {
                command: request.command,
                reason: request.reason,
                status: result.status(),
                stdout: result.stdout().to_string(),
                stderr: result.stderr().to_string(),
                stdout_truncated: result.stdout_truncated(),
                stderr_truncated: result.stderr_truncated(),
                duration_ms: start.elapsed().as_millis(),
                redaction_status: "redacted",
            }),
            TerminalToolOutcome::ApprovalRequired => TestRunOutcome::ApprovalRequired,
            TerminalToolOutcome::Denied => TestRunOutcome::Denied,
            TerminalToolOutcome::Blocked(reason) => TestRunOutcome::Blocked(reason),
        }
    }
}

fn select(command: &DetectedTestCommand) -> TestCommandSelection {
    TestCommandSelection::Selected(SelectedTestCommand {
        command: command.command().to_string(),
        reason: format!(
            "selected_test_command:{} from {} confidence={:?}",
            command.command(),
            command.source_path().display(),
            command.confidence()
        ),
        confidence: command.confidence(),
    })
}

fn clarify(commands: Vec<&DetectedTestCommand>, reason: &str) -> TestCommandSelection {
    TestCommandSelection::ClarificationRequired {
        candidates: commands
            .iter()
            .map(|command| command.command().to_string())
            .collect(),
        reason: reason.to_string(),
    }
}
