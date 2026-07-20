use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::IterativeToolCall;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IterativeApprovalDecision {
    Approved,
    Denied,
    Expired,
}

impl IterativeApprovalDecision {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IterativeApproval {
    call_id: String,
    payload_fingerprint: String,
    decision: IterativeApprovalDecision,
}

impl IterativeApproval {
    #[must_use]
    pub fn approved(call_id: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self::new(call_id, fingerprint, IterativeApprovalDecision::Approved)
    }

    #[must_use]
    pub fn denied(call_id: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self::new(call_id, fingerprint, IterativeApprovalDecision::Denied)
    }

    #[must_use]
    pub fn expired(call_id: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self::new(call_id, fingerprint, IterativeApprovalDecision::Expired)
    }

    fn new(
        call_id: impl Into<String>,
        payload_fingerprint: impl Into<String>,
        decision: IterativeApprovalDecision,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            payload_fingerprint: payload_fingerprint.into(),
            decision,
        }
    }

    pub(crate) fn matches(&self, pending: &PendingToolApproval) -> bool {
        self.call_id == pending.call_id && self.payload_fingerprint == pending.payload_fingerprint
    }

    pub(crate) fn decision(&self) -> IterativeApprovalDecision {
        self.decision
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PendingToolApproval {
    call: IterativeToolCall,
    call_id: String,
    payload_fingerprint: String,
}

impl PendingToolApproval {
    #[must_use]
    pub(crate) fn new(call: IterativeToolCall) -> Self {
        Self {
            call_id: call.id().to_string(),
            payload_fingerprint: payload_fingerprint(&call),
            call,
        }
    }

    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    #[must_use]
    pub fn payload_fingerprint(&self) -> &str {
        &self.payload_fingerprint
    }

    pub fn call(&self) -> &IterativeToolCall {
        &self.call
    }
}

fn payload_fingerprint(call: &IterativeToolCall) -> String {
    let digest = Sha256::digest(call.signature().as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
