use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{IterativeLoopState, ToolObservation};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IterativeLoopStatus {
    Running,
    WaitingForApproval,
    Completed,
    Blocked,
    Failed,
    Cancelled,
    Exhausted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IterativeStopReason {
    FinalResponse,
    ModelBlocked(String),
    Clarification {
        question: String,
        blocked_on: String,
    },
    ModelFailure(String),
    Cancelled(String),
    MaxTurns,
    MaxToolCalls,
    MaxDuration,
    RepeatedToolFailure,
    DuplicateToolCall(String),
    InvalidFinalResponse(String),
    UnsupportedTestClaim,
    ApprovalRequired,
    ApprovalDenied,
    ApprovalExpired,
    ApprovalPayloadMismatch,
}

impl IterativeStopReason {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::FinalResponse => "final_response",
            Self::ModelBlocked(_) => "model_blocked",
            Self::Clarification { .. } => "clarification_required",
            Self::ModelFailure(_) => "model_failure",
            Self::Cancelled(_) => "cancelled",
            Self::MaxTurns => "max_turns",
            Self::MaxToolCalls => "max_tool_calls",
            Self::MaxDuration => "max_duration",
            Self::RepeatedToolFailure => "repeated_tool_failure",
            Self::DuplicateToolCall(_) => "duplicate_tool_call",
            Self::InvalidFinalResponse(_) => "invalid_final_response",
            Self::UnsupportedTestClaim => "unsupported_test_claim",
            Self::ApprovalRequired => "approval_required",
            Self::ApprovalDenied => "approval_denied",
            Self::ApprovalExpired => "approval_expired",
            Self::ApprovalPayloadMismatch => "approval_payload_mismatch",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IterativeToolCall {
    id: String,
    name: String,
    arguments: Value,
}

impl IterativeToolCall {
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn arguments(&self) -> &Value {
        &self.arguments
    }

    pub(crate) fn signature(&self) -> String {
        format!("{}:{}", self.name, self.arguments)
    }

    pub(crate) fn failure_signature(&self, error: &str) -> String {
        format!("{}:{error}", self.signature())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IterativeLoopEvent {
    ModelTurn {
        ordinal: usize,
    },
    ModelProtocolRetry {
        ordinal: usize,
        reason: String,
    },
    ToolRequested {
        call: IterativeToolCall,
    },
    ToolObserved {
        observation: ToolObservation,
    },
    ApprovalRequired {
        call_id: String,
        payload_fingerprint: String,
    },
    ApprovalResolved {
        call_id: String,
        decision: String,
    },
    Completed {
        response: String,
    },
    Stopped {
        reason: IterativeStopReason,
    },
}

impl IterativeLoopEvent {
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IterativeModelDecision {
    ToolCall(IterativeToolCall),
    FinalResponse(String),
    Blocked(String),
    Clarification {
        question: String,
        blocked_on: String,
    },
}

impl IterativeModelDecision {
    #[must_use]
    pub fn tool_call(call: IterativeToolCall) -> Self {
        Self::ToolCall(call)
    }

    #[must_use]
    pub fn final_response(response: impl Into<String>) -> Self {
        Self::FinalResponse(response.into())
    }

    #[must_use]
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self::Blocked(reason.into())
    }

    #[must_use]
    pub fn clarification(question: impl Into<String>, blocked_on: impl Into<String>) -> Self {
        Self::Clarification {
            question: question.into(),
            blocked_on: blocked_on.into(),
        }
    }
}

pub trait IterativeModelAdapter {
    fn decide(&mut self, state: &IterativeLoopState) -> Result<IterativeModelDecision, String>;
}

pub trait IterativeToolExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String>;

    fn execute_approved(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        self.execute(call)
    }
}
