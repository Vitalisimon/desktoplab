use serde_json::{Value, json};

use crate::verification_evidence::{VerificationEvidence, VerificationState};
use desktoplab_backends::BackendModelCapabilities;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReadinessState {
    runtime_id: Option<String>,
    model_id: Option<String>,
    runtime_verification: VerificationEvidence,
    model_verification: VerificationEvidence,
    model_capabilities: Option<BackendModelCapabilities>,
}

impl Default for BackendReadinessState {
    fn default() -> Self {
        Self {
            runtime_id: None,
            model_id: None,
            runtime_verification: VerificationEvidence::missing(),
            model_verification: VerificationEvidence::missing(),
            model_capabilities: None,
        }
    }
}

impl BackendReadinessState {
    #[must_use]
    pub fn select(mut self, runtime_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        self.runtime_id = Some(runtime_id.into());
        self.model_id = Some(model_id.into());
        self.runtime_verification = VerificationEvidence::missing();
        self.model_verification = VerificationEvidence::missing();
        self.model_capabilities = None;
        self
    }

    pub fn mark_runtime_verified(
        &mut self,
        runtime_id: impl Into<String>,
        evidence: impl Into<String>,
    ) {
        self.runtime_id = Some(runtime_id.into());
        self.runtime_verification = VerificationEvidence::verified(evidence);
    }

    pub fn mark_runtime_blocked(
        &mut self,
        runtime_id: impl Into<String>,
        evidence: impl Into<String>,
    ) {
        self.runtime_id = Some(runtime_id.into());
        self.runtime_verification = VerificationEvidence::blocked(evidence);
    }

    pub fn mark_model_verified(
        &mut self,
        runtime_id: impl Into<String>,
        model_id: impl Into<String>,
        evidence: impl Into<String>,
    ) {
        let runtime_id = runtime_id.into();
        let model_id = model_id.into();
        if self.runtime_id.as_deref() != Some(runtime_id.as_str())
            || self.model_id.as_deref() != Some(model_id.as_str())
        {
            self.model_capabilities = None;
        }
        self.runtime_id = Some(runtime_id);
        self.model_id = Some(model_id);
        self.model_verification = VerificationEvidence::verified(evidence);
    }

    pub fn mark_model_capabilities(&mut self, mut capabilities: BackendModelCapabilities) {
        if capabilities.tool_protocol_certification().is_none()
            && let Some(certification) = self
                .model_capabilities
                .as_ref()
                .filter(|current| current.fingerprint() == capabilities.fingerprint())
                .and_then(BackendModelCapabilities::tool_protocol_certification)
                .cloned()
        {
            capabilities = capabilities.with_tool_protocol_certification(certification);
        }
        self.model_capabilities = Some(capabilities);
    }

    #[must_use]
    pub fn model_capabilities(&self) -> Option<&BackendModelCapabilities> {
        self.model_capabilities.as_ref()
    }

    pub fn mark_model_blocked(
        &mut self,
        runtime_id: impl Into<String>,
        model_id: impl Into<String>,
        evidence: impl Into<String>,
    ) {
        self.runtime_id = Some(runtime_id.into());
        self.model_id = Some(model_id.into());
        self.model_verification = VerificationEvidence::blocked(evidence);
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.runtime_verification.state == VerificationState::Verified
            && self.model_verification.state == VerificationState::Verified
    }

    #[must_use]
    pub fn runtime_verified(&self) -> bool {
        self.runtime_verification.state == VerificationState::Verified
    }

    pub fn runtime_verified_for(&self, runtime_id: &str) -> bool {
        self.runtime_verified() && self.runtime_id.as_deref() == Some(runtime_id)
    }

    pub fn model_verified(&self) -> bool {
        self.model_verification.state == VerificationState::Verified
    }

    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    #[must_use]
    pub fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }

    #[must_use]
    pub fn blocked_reason(&self) -> Option<&'static str> {
        match (
            self.runtime_verification.state == VerificationState::Verified,
            self.model_verification.state == VerificationState::Verified,
        ) {
            (true, true) => None,
            (false, false) => Some("runtime_and_model_not_verified"),
            (false, true) => Some("runtime_not_verified"),
            (true, false) => Some("model_not_verified"),
        }
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "state":if self.is_ready() {"ready"} else {"blocked"},
            "runtimeId":self.runtime_id,
            "modelId":self.model_id,
            "runtimeVerification":self.runtime_verification.to_json(),
            "modelVerification":self.model_verification.to_json(),
            "modelCapabilities":self.model_capabilities.as_ref().map(BackendModelCapabilities::to_json),
            "blockedReason":self.blocked_reason(),
            "lastEvidence":self.last_evidence(),
            "lastVerifiedAt":self.last_verified_at()
        })
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Self {
        Self {
            runtime_id: value
                .get("runtimeId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            model_id: value
                .get("modelId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            runtime_verification: value
                .get("runtimeVerification")
                .map(VerificationEvidence::from_json)
                .unwrap_or_else(VerificationEvidence::missing),
            model_verification: value
                .get("modelVerification")
                .map(VerificationEvidence::from_json)
                .unwrap_or_else(VerificationEvidence::missing),
            model_capabilities: value
                .get("modelCapabilities")
                .and_then(BackendModelCapabilities::from_json),
        }
    }

    fn last_evidence(&self) -> Option<&str> {
        self.model_verification
            .evidence
            .as_deref()
            .or(self.runtime_verification.evidence.as_deref())
    }

    fn last_verified_at(&self) -> Option<&str> {
        self.model_verification
            .verified_at
            .as_deref()
            .or(self.runtime_verification.verified_at.as_deref())
    }
}
