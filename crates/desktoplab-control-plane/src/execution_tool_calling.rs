use desktoplab_agent_engine::{
    IterativeToolCall, ProviderToolCallNormalizer, ToolCallNormalizationError,
};
use desktoplab_backends::{BackendModelCapabilities, ModelToolProtocolKind};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendToolProtocolClass {
    NativeTool,
    ConstrainedJson,
    ChatOnly,
}

impl BackendToolProtocolClass {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeTool => "native_tool",
            Self::ConstrainedJson => "constrained_json",
            Self::ChatOnly => "chat_only",
        }
    }

    #[must_use]
    pub fn supports_full_coding_agent(self) -> bool {
        self != Self::ChatOnly
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolProtocolError {
    ChatOnly,
    MalformedOutput,
    InvalidCall(ToolCallNormalizationError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendToolProtocolHealth {
    declared: BackendToolProtocolClass,
    effective: BackendToolProtocolClass,
    invalid_actions: usize,
    downgrade_threshold: usize,
}

impl BackendToolProtocolHealth {
    #[must_use]
    pub fn new(declared: BackendToolProtocolClass, downgrade_threshold: usize) -> Self {
        Self {
            declared,
            effective: declared,
            invalid_actions: 0,
            downgrade_threshold: downgrade_threshold.max(1),
        }
    }

    pub fn record_invalid_action(&mut self) {
        self.invalid_actions += 1;
        if self.invalid_actions >= self.downgrade_threshold {
            self.effective = BackendToolProtocolClass::ChatOnly;
        }
    }

    #[must_use]
    pub fn effective(&self) -> BackendToolProtocolClass {
        self.effective
    }

    #[must_use]
    pub fn invalid_actions(&self) -> usize {
        self.invalid_actions
    }

    #[must_use]
    pub fn declared(&self) -> BackendToolProtocolClass {
        self.declared
    }
}

#[must_use]
pub fn backend_tool_protocol_class(backend_id: &str) -> BackendToolProtocolClass {
    match backend_id {
        "backend.mlx-lm" => BackendToolProtocolClass::ConstrainedJson,
        "backend.codex" | "backend.openai" => BackendToolProtocolClass::ChatOnly,
        _ => BackendToolProtocolClass::NativeTool,
    }
}

#[must_use]
pub fn model_tool_protocol_class(
    backend_id: &str,
    capabilities: Option<&BackendModelCapabilities>,
) -> BackendToolProtocolClass {
    if backend_id != "backend.ollama" {
        return backend_tool_protocol_class(backend_id);
    }
    match capabilities.and_then(BackendModelCapabilities::tool_protocol_kind) {
        Some(ModelToolProtocolKind::NativeTools) => BackendToolProtocolClass::NativeTool,
        Some(ModelToolProtocolKind::ConstrainedJson) => BackendToolProtocolClass::ConstrainedJson,
        None => BackendToolProtocolClass::ChatOnly,
    }
}

pub fn normalize_backend_tool_output(
    output: &str,
    class: BackendToolProtocolClass,
    call_id: &str,
) -> Result<IterativeToolCall, ToolProtocolError> {
    if class == BackendToolProtocolClass::ChatOnly {
        return Err(ToolProtocolError::ChatOnly);
    }
    let mut value =
        serde_json::from_str::<Value>(output).map_err(|_| ToolProtocolError::MalformedOutput)?;
    let object = value
        .as_object_mut()
        .ok_or(ToolProtocolError::MalformedOutput)?;
    if !object.contains_key("id") {
        object.insert("id".to_string(), Value::String(call_id.to_string()));
    }
    ProviderToolCallNormalizer::default()
        .from_provider_value(&value)
        .map_err(ToolProtocolError::InvalidCall)
}
