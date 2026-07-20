use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerificationEvidence {
    pub(crate) state: VerificationState,
    pub(crate) evidence: Option<String>,
    pub(crate) verified_at: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerificationState {
    Missing,
    Verified,
    Blocked,
}

impl VerificationEvidence {
    pub(crate) fn missing() -> Self {
        Self {
            state: VerificationState::Missing,
            evidence: None,
            verified_at: None,
        }
    }

    pub(crate) fn verified(evidence: impl Into<String>) -> Self {
        Self {
            state: VerificationState::Verified,
            evidence: Some(evidence.into()),
            verified_at: Some("1970-01-01T00:00:00Z".to_string()),
        }
    }

    pub(crate) fn blocked(evidence: impl Into<String>) -> Self {
        Self {
            state: VerificationState::Blocked,
            evidence: Some(evidence.into()),
            verified_at: None,
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "state":self.state.as_str(),
            "evidence":self.evidence,
            "verifiedAt":self.verified_at
        })
    }

    pub(crate) fn from_json(value: &Value) -> Self {
        Self {
            state: value
                .get("state")
                .and_then(Value::as_str)
                .map(VerificationState::from_str)
                .unwrap_or(VerificationState::Missing),
            evidence: value
                .get("evidence")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            verified_at: value
                .get("verifiedAt")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        }
    }
}

impl VerificationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Verified => "verified",
            Self::Blocked => "blocked",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "verified" => Self::Verified,
            "blocked" => Self::Blocked,
            _ => Self::Missing,
        }
    }
}
