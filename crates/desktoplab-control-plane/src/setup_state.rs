use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupState {
    phase: SetupPhase,
    runtime_id: Option<String>,
    model_id: Option<String>,
    runtime_ready: bool,
    model_ready: bool,
    blocked_reason: Option<String>,
    last_verified_at: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupPhase {
    NotStarted,
    InProgress,
    Ready,
    Blocked,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            phase: SetupPhase::NotStarted,
            runtime_id: None,
            model_id: None,
            runtime_ready: false,
            model_ready: false,
            blocked_reason: None,
            last_verified_at: None,
        }
    }
}

impl SetupState {
    #[must_use]
    pub fn accept(runtime_id: String, model_id: String) -> Self {
        Self {
            phase: SetupPhase::InProgress,
            runtime_id: Some(runtime_id),
            model_id: Some(model_id),
            runtime_ready: false,
            model_ready: false,
            blocked_reason: None,
            last_verified_at: None,
        }
    }

    #[must_use]
    pub fn complete(self, runtime_ready: bool, model_ready: bool) -> Self {
        if runtime_ready && model_ready {
            return Self {
                phase: SetupPhase::Ready,
                runtime_ready,
                model_ready,
                blocked_reason: None,
                last_verified_at: Some("1970-01-01T00:00:00Z".to_string()),
                ..self
            };
        }
        Self {
            phase: SetupPhase::Blocked,
            runtime_ready,
            model_ready,
            blocked_reason: Some(blocked_reason(runtime_ready, model_ready).to_string()),
            ..self
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.phase == SetupPhase::Ready && self.runtime_ready && self.model_ready
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "state":self.phase.as_str(),
            "runtimeId":self.runtime_id,
            "modelId":self.model_id,
            "runtimeReady":self.runtime_ready,
            "modelReady":self.model_ready,
            "blockedReason":self.blocked_reason,
            "lastVerifiedAt":self.last_verified_at
        })
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Self {
        let phase = value
            .get("state")
            .and_then(Value::as_str)
            .map(SetupPhase::from_str)
            .unwrap_or(SetupPhase::Blocked);
        Self {
            phase,
            runtime_id: value
                .get("runtimeId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            model_id: value
                .get("modelId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            runtime_ready: value
                .get("runtimeReady")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            model_ready: value
                .get("modelReady")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            blocked_reason: value
                .get("blockedReason")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            last_verified_at: value
                .get("lastVerifiedAt")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        }
    }
}

impl SetupPhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Ready => "ready",
            Self::Blocked => "blocked",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "not_started" => Self::NotStarted,
            "in_progress" => Self::InProgress,
            "ready" => Self::Ready,
            "blocked" => Self::Blocked,
            _ => Self::Blocked,
        }
    }
}

fn blocked_reason(runtime_ready: bool, model_ready: bool) -> &'static str {
    match (runtime_ready, model_ready) {
        (false, false) => "runtime_and_model_not_verified",
        (false, true) => "runtime_not_verified",
        (true, false) => "model_not_verified",
        (true, true) => "",
    }
}
