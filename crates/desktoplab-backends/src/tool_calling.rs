use serde_json::{Value, json};

use crate::ProviderCompatibilityProfile;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendToolSchema {
    name: String,
    description: String,
    parameters: Value,
}

impl BackendToolSchema {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    pub(crate) fn as_provider_tool(&self) -> Value {
        json!({
            "type":"function",
            "function":{
                "name":self.name,
                "description":self.description,
                "parameters":self.parameters
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendToolCall {
    id: Option<String>,
    name: String,
    arguments: Value,
    recovery_reason: Option<String>,
}

impl BackendToolCall {
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn arguments(&self) -> &Value {
        &self.arguments
    }

    #[must_use]
    pub fn recovery_reason(&self) -> Option<&str> {
        self.recovery_reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendToolCallEvidence {
    backend_id: String,
    model_id: String,
    endpoint: Option<String>,
    native_tool_calls: bool,
    structured_output_supported: bool,
    streaming_supported: bool,
    fallback_reason: Option<String>,
}

impl BackendToolCallEvidence {
    #[must_use]
    pub fn native(
        backend_id: impl Into<String>,
        model_id: impl Into<String>,
        endpoint: impl Into<String>,
        streaming_supported: bool,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            model_id: model_id.into(),
            endpoint: Some(endpoint.into()),
            native_tool_calls: true,
            structured_output_supported: true,
            streaming_supported,
            fallback_reason: None,
        }
    }

    #[must_use]
    pub fn fallback(
        backend_id: impl Into<String>,
        model_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            model_id: model_id.into(),
            endpoint: None,
            native_tool_calls: false,
            structured_output_supported: true,
            streaming_supported: false,
            fallback_reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn native_tool_calls(&self) -> bool {
        self.native_tool_calls
    }

    #[must_use]
    pub fn streaming_supported(&self) -> bool {
        self.streaming_supported
    }

    #[must_use]
    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendToolResponse {
    assistant_text: Option<String>,
    reasoning_text: Option<String>,
    tool_calls: Vec<BackendToolCall>,
    evidence: BackendToolCallEvidence,
    protocol_error: Option<String>,
}

impl BackendToolResponse {
    #[must_use]
    pub fn assistant_text(&self) -> Option<&str> {
        self.assistant_text.as_deref()
    }

    #[must_use]
    pub fn tool_calls(&self) -> &[BackendToolCall] {
        &self.tool_calls
    }

    #[must_use]
    pub fn reasoning_text(&self) -> Option<&str> {
        self.reasoning_text.as_deref()
    }

    #[must_use]
    pub fn protocol_error(&self) -> Option<&str> {
        self.protocol_error.as_deref()
    }

    #[must_use]
    pub fn evidence(&self) -> &BackendToolCallEvidence {
        &self.evidence
    }
}

#[must_use]
pub fn provider_tools(tools: &[BackendToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(BackendToolSchema::as_provider_tool)
        .collect()
}

pub fn parse_constrained_tool_text(content: &str) -> Result<String, String> {
    let value = serde_json::from_str::<Value>(content)
        .map_err(|_| "provider_constrained_tool_invalid_json".to_string())?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "provider_constrained_tool_missing_name".to_string())?;
    let arguments = value
        .get("arguments")
        .filter(|arguments| arguments.is_object())
        .ok_or_else(|| "provider_tool_arguments_must_be_object".to_string())?;
    Ok(json!({
        "assistantMessage":"",
        "tool":name,
        "arguments":arguments
    })
    .to_string())
}

#[must_use]
pub fn parse_ollama_tool_response(
    response: &Value,
    evidence: BackendToolCallEvidence,
) -> BackendToolResponse {
    parse_tool_response(
        response,
        &ProviderCompatibilityProfile::ollama_chat(),
        evidence,
    )
}

#[must_use]
pub fn parse_openai_compatible_tool_response(
    response: &Value,
    evidence: BackendToolCallEvidence,
) -> BackendToolResponse {
    parse_tool_response(
        response,
        &ProviderCompatibilityProfile::openai_chat_completions(),
        evidence,
    )
}

#[must_use]
pub fn parse_tool_response(
    response: &Value,
    profile: &ProviderCompatibilityProfile,
    evidence: BackendToolCallEvidence,
) -> BackendToolResponse {
    let message = profile.message(response);
    let (tool_calls, mut protocol_error) = parse_tool_calls(message);
    if tool_calls.len() > profile.max_tool_calls_per_turn() {
        protocol_error = Some("parallel_tool_calls_unsupported".to_string());
    }
    BackendToolResponse {
        assistant_text: message["content"].as_str().map(ToString::to_string),
        reasoning_text: profile.reasoning_text(message),
        tool_calls,
        evidence,
        protocol_error,
    }
}

fn parse_tool_calls(message: &Value) -> (Vec<BackendToolCall>, Option<String>) {
    let Some(raw_calls) = message.get("tool_calls") else {
        return (Vec::new(), None);
    };
    let Some(calls) = raw_calls.as_array() else {
        return (
            Vec::new(),
            Some("provider_tool_calls_must_be_array".to_string()),
        );
    };
    match calls
        .iter()
        .map(parse_tool_call)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(calls) => (calls, None),
        Err(error) => (Vec::new(), Some(error)),
    }
}

fn parse_tool_call(value: &Value) -> Result<BackendToolCall, String> {
    let function = &value["function"];
    let name = function["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "provider_tool_call_missing_name".to_string())?
        .to_string();
    let arguments = parse_arguments(&function["arguments"])?;
    Ok(BackendToolCall {
        id: value["id"].as_str().map(ToString::to_string),
        name,
        arguments,
        recovery_reason: None,
    })
}

fn parse_arguments(arguments: &Value) -> Result<Value, String> {
    let value = if let Some(raw) = arguments.as_str() {
        serde_json::from_str(raw).map_err(|_| "provider_tool_arguments_invalid_json".to_string())?
    } else {
        arguments.clone()
    };
    if !value.is_object() {
        return Err("provider_tool_arguments_must_be_object".to_string());
    }
    Ok(value)
}
