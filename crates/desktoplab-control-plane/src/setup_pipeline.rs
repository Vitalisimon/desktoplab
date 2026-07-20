use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupPipeline {
    state: SetupPipelineState,
    runtime_id: Option<String>,
    model_id: Option<String>,
    blocked_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupPipelineState {
    NotStarted,
    Selected,
    RuntimeDetecting,
    RuntimeInstalling,
    RuntimeVerifying,
    ModelDownloading,
    ModelVerifying,
    Ready,
    Blocked,
}

impl Default for SetupPipeline {
    fn default() -> Self {
        Self {
            state: SetupPipelineState::NotStarted,
            runtime_id: None,
            model_id: None,
            blocked_reason: None,
        }
    }
}

impl SetupPipeline {
    #[must_use]
    pub fn select(runtime_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            state: SetupPipelineState::Selected,
            runtime_id: Some(runtime_id.into()),
            model_id: Some(model_id.into()),
            blocked_reason: None,
        }
    }

    #[must_use]
    pub fn advance(mut self, state: SetupPipelineState) -> Self {
        if !matches!(
            state,
            SetupPipelineState::Blocked | SetupPipelineState::NotStarted
        ) {
            self.state = state;
            self.blocked_reason = None;
        }
        self
    }

    #[must_use]
    pub fn block(mut self, reason: impl Into<String>) -> Self {
        self.state = SetupPipelineState::Blocked;
        self.blocked_reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn ready(mut self) -> Self {
        self.state = SetupPipelineState::Ready;
        self.blocked_reason = None;
        self
    }

    #[must_use]
    pub fn state(&self) -> SetupPipelineState {
        self.state
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "state":self.state.as_str(),
            "runtimeId":self.runtime_id,
            "modelId":self.model_id,
            "blockedReason":self.blocked_reason
        })
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Self {
        Self {
            state: value
                .get("state")
                .and_then(Value::as_str)
                .map(SetupPipelineState::from_str)
                .unwrap_or(SetupPipelineState::Blocked),
            runtime_id: value
                .get("runtimeId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            model_id: value
                .get("modelId")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            blocked_reason: value
                .get("blockedReason")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        }
    }
}

impl SetupPipelineState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Selected => "selected",
            Self::RuntimeDetecting => "runtime_detecting",
            Self::RuntimeInstalling => "runtime_installing",
            Self::RuntimeVerifying => "runtime_verifying",
            Self::ModelDownloading => "model_downloading",
            Self::ModelVerifying => "model_verifying",
            Self::Ready => "ready",
            Self::Blocked => "blocked",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "not_started" => Self::NotStarted,
            "selected" => Self::Selected,
            "runtime_detecting" => Self::RuntimeDetecting,
            "runtime_installing" => Self::RuntimeInstalling,
            "runtime_verifying" => Self::RuntimeVerifying,
            "model_downloading" => Self::ModelDownloading,
            "model_verifying" => Self::ModelVerifying,
            "ready" => Self::Ready,
            _ => Self::Blocked,
        }
    }
}
