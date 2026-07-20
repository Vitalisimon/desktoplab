#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentFailureKind {
    ModelRefusal,
    ToolFailure,
    TestFailure,
    Timeout,
    PolicyDenial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureObservation {
    kind: AgentFailureKind,
    summary: String,
}

impl FailureObservation {
    #[must_use]
    pub fn new(kind: AgentFailureKind, summary: impl Into<String>) -> Self {
        Self {
            kind,
            summary: summary.into(),
        }
    }

    #[must_use]
    pub fn test_failed(summary: impl Into<String>) -> Self {
        Self::new(AgentFailureKind::TestFailure, summary)
    }

    #[must_use]
    pub fn kind(&self) -> AgentFailureKind {
        self.kind
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryAttempt {
    observation: FailureObservation,
    patch_summary: Option<String>,
    rerun_summary: Option<String>,
}

impl RetryAttempt {
    #[must_use]
    pub fn from_observation(observation: FailureObservation) -> Self {
        Self {
            observation,
            patch_summary: None,
            rerun_summary: None,
        }
    }

    #[must_use]
    pub fn with_patch_summary(mut self, summary: impl Into<String>) -> Self {
        self.patch_summary = Some(summary.into());
        self
    }

    #[must_use]
    pub fn with_rerun_summary(mut self, summary: impl Into<String>) -> Self {
        self.rerun_summary = Some(summary.into());
        self
    }

    #[must_use]
    pub fn observation(&self) -> &FailureObservation {
        &self.observation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryDecision {
    Retry,
    Stop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryEvaluation {
    decision: RetryDecision,
    reason: String,
    truthful_summary: String,
}

impl RetryEvaluation {
    #[must_use]
    pub fn decision(&self) -> RetryDecision {
        self.decision
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    #[must_use]
    pub fn truthful_summary(&self) -> &str {
        &self.truthful_summary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    max_retries: usize,
}

impl RetryPolicy {
    #[must_use]
    pub fn new(max_retries: usize) -> Self {
        Self { max_retries }
    }

    #[must_use]
    pub fn evaluate(
        &self,
        attempts: &[RetryAttempt],
        latest: &FailureObservation,
    ) -> RetryEvaluation {
        if !retryable(latest.kind()) {
            return RetryEvaluation {
                decision: RetryDecision::Stop,
                reason: failure_reason(latest.kind()).to_string(),
                truthful_summary: format!(
                    "Stopped without retry: {}. Latest observation: {}",
                    failure_reason(latest.kind()),
                    latest.summary()
                ),
            };
        }
        if attempts.len() >= self.max_retries {
            return RetryEvaluation {
                decision: RetryDecision::Stop,
                reason: "max_retry_count_reached".to_string(),
                truthful_summary: format!(
                    "Validation is still failing after {} retry attempt(s). Latest observation: {}",
                    attempts.len(),
                    latest.summary()
                ),
            };
        }
        RetryEvaluation {
            decision: RetryDecision::Retry,
            reason: "retryable_validation_failure".to_string(),
            truthful_summary: format!(
                "Retry allowed for {:?}: {}",
                latest.kind(),
                latest.summary()
            ),
        }
    }
}

fn retryable(kind: AgentFailureKind) -> bool {
    matches!(
        kind,
        AgentFailureKind::ToolFailure | AgentFailureKind::TestFailure | AgentFailureKind::Timeout
    )
}

fn failure_reason(kind: AgentFailureKind) -> &'static str {
    match kind {
        AgentFailureKind::ModelRefusal => "model_refusal",
        AgentFailureKind::ToolFailure => "tool_failure",
        AgentFailureKind::TestFailure => "test_failure",
        AgentFailureKind::Timeout => "timeout",
        AgentFailureKind::PolicyDenial => "policy_denial",
    }
}
