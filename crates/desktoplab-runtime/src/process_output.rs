use super::{ProcessCommand, bound, redact};

#[derive(Clone, Eq, PartialEq)]
pub struct ProcessOutput {
    exit_code: Option<i32>,
    stdout: String,
    stdout_for_checks: String,
    stderr: String,
    evidence: ProcessCommand,
}

impl std::fmt::Debug for ProcessOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessOutput")
            .field("exit_code", &self.exit_code)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .field("evidence", &self.evidence)
            .finish()
    }
}

impl ProcessOutput {
    #[must_use]
    pub fn new(
        exit_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        evidence: ProcessCommand,
    ) -> Self {
        let stdout = redact(&stdout.into());
        Self {
            exit_code,
            stdout: bound(&stdout),
            stdout_for_checks: stdout.chars().take(16_384).collect(),
            stderr: bound(&redact(&stderr.into())),
            evidence,
        }
    }
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }
    pub(crate) fn stdout_for_checks(&self) -> &str {
        &self.stdout_for_checks
    }
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }
    #[must_use]
    pub fn evidence(&self) -> &ProcessCommand {
        &self.evidence
    }
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }
}
