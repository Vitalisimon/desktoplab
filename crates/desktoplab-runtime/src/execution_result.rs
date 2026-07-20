#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeExecutionState {
    Completed,
    Blocked,
    ExternalGuided,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallExecutionResult {
    state: RuntimeExecutionState,
    verification_state: String,
    evidence: String,
    remediation: String,
    desktoplab_started_runtime: bool,
}

impl RuntimeInstallExecutionResult {
    #[must_use]
    pub fn state(&self) -> RuntimeExecutionState {
        self.state
    }

    #[must_use]
    pub fn verification_state(&self) -> &str {
        &self.verification_state
    }

    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }

    #[must_use]
    pub fn remediation(&self) -> &str {
        &self.remediation
    }

    #[must_use]
    pub const fn desktoplab_started_runtime(&self) -> bool {
        self.desktoplab_started_runtime
    }

    pub(crate) fn completed(evidence: impl Into<String>) -> Self {
        Self::new(
            RuntimeExecutionState::Completed,
            "verified",
            evidence,
            "",
            false,
        )
    }

    pub(crate) fn completed_after_desktoplab_start(evidence: impl Into<String>) -> Self {
        Self::new(
            RuntimeExecutionState::Completed,
            "verified",
            evidence,
            "",
            true,
        )
    }

    pub(crate) fn blocked(evidence: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self::new(
            RuntimeExecutionState::Blocked,
            "blocked",
            evidence,
            remediation,
            false,
        )
    }

    pub fn blocked_with_state(
        verification_state: impl Into<String>,
        evidence: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self::new(
            RuntimeExecutionState::Blocked,
            verification_state,
            evidence,
            remediation,
            false,
        )
    }

    pub(crate) fn failed(
        verification_state: impl Into<String>,
        evidence: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self::new(
            RuntimeExecutionState::Failed,
            verification_state,
            evidence,
            remediation,
            false,
        )
    }

    pub(crate) fn external_guided(remediation: impl Into<String>) -> Self {
        Self::new(
            RuntimeExecutionState::ExternalGuided,
            "requires_external_app",
            "lm-studio guided setup",
            remediation,
            false,
        )
    }

    fn new(
        state: RuntimeExecutionState,
        verification_state: impl Into<String>,
        evidence: impl Into<String>,
        remediation: impl Into<String>,
        desktoplab_started_runtime: bool,
    ) -> Self {
        Self {
            state,
            verification_state: verification_state.into(),
            evidence: evidence.into(),
            remediation: remediation.into(),
            desktoplab_started_runtime,
        }
    }
}
