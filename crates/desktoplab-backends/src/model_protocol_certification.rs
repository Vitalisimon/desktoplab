use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelProtocolCertificationState {
    Certified,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolProtocolKind {
    NativeTools,
    ConstrainedJson,
}

impl ModelToolProtocolKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeTools => "native_tools.v1",
            Self::ConstrainedJson => "constrained_json.v1",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "native_tools.v1" => Some(Self::NativeTools),
            "constrained_json.v1" => Some(Self::ConstrainedJson),
            _ => None,
        }
    }
}

impl ModelProtocolCertificationState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Certified => "certified",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelToolProtocolCertification {
    capability_fingerprint: String,
    protocol: String,
    state: ModelProtocolCertificationState,
    failure_reason: Option<String>,
}

impl ModelToolProtocolCertification {
    #[must_use]
    pub fn certified(fingerprint: impl Into<String>) -> Self {
        Self::certified_as(fingerprint, ModelToolProtocolKind::NativeTools)
    }

    #[must_use]
    pub fn certified_as(fingerprint: impl Into<String>, protocol: ModelToolProtocolKind) -> Self {
        Self {
            capability_fingerprint: fingerprint.into(),
            protocol: protocol.as_str().to_string(),
            state: ModelProtocolCertificationState::Certified,
            failure_reason: None,
        }
    }

    #[must_use]
    pub fn failed(fingerprint: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            capability_fingerprint: fingerprint.into(),
            protocol: "unclassified".to_string(),
            state: ModelProtocolCertificationState::Failed,
            failure_reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn is_certified_for(&self, fingerprint: &str) -> bool {
        self.is_for(fingerprint) && self.state == ModelProtocolCertificationState::Certified
    }

    #[must_use]
    pub fn is_for(&self, fingerprint: &str) -> bool {
        self.capability_fingerprint == fingerprint
    }

    #[must_use]
    pub fn state(&self) -> ModelProtocolCertificationState {
        self.state
    }

    #[must_use]
    pub fn protocol(&self) -> Option<ModelToolProtocolKind> {
        ModelToolProtocolKind::from_str(&self.protocol)
    }

    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "capabilityFingerprint":self.capability_fingerprint,
            "protocol":self.protocol,
            "state":self.state.as_str(),
            "failureReason":self.failure_reason
        })
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Option<Self> {
        let state = match value.get("state")?.as_str()? {
            "certified" => ModelProtocolCertificationState::Certified,
            "failed" => ModelProtocolCertificationState::Failed,
            _ => return None,
        };
        Some(Self {
            capability_fingerprint: value.get("capabilityFingerprint")?.as_str()?.to_string(),
            protocol: value.get("protocol")?.as_str()?.to_string(),
            state,
            failure_reason: value
                .get("failureReason")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        })
    }
}
