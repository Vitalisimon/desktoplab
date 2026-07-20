use desktoplab_agent_session::AgentSession;
use desktoplab_execution_router::ExecutionRouteCandidate;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::{
    BackendModelCapabilities, BackendPrompt, BackendToolCallEvidence, ModelCapabilityState,
    ModelToolProtocolKind, backend_response_to_agent_text, parse_constrained_tool_text,
    parse_ollama_tool_response, provider_tools,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendModelInventory {
    models: Vec<String>,
}

impl BackendModelInventory {
    #[must_use]
    pub fn available(models: &[&str]) -> Self {
        Self {
            models: models.iter().map(ToString::to_string).collect(),
        }
    }

    pub(crate) fn contains(&self, model: &str) -> bool {
        self.models.iter().any(|existing| existing == model)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendExecutionResult {
    ready: bool,
    reason: Option<String>,
}

impl BackendExecutionResult {
    #[must_use]
    pub(crate) fn ready() -> Self {
        Self {
            ready: true,
            reason: None,
        }
    }

    #[must_use]
    pub(crate) fn blocked(reason: impl Into<String>) -> Self {
        Self {
            ready: false,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaExecutionBackend {
    inventory: BackendModelInventory,
    model_capabilities: BTreeMap<String, BackendModelCapabilities>,
}

impl OllamaExecutionBackend {
    #[must_use]
    pub fn new(inventory: BackendModelInventory) -> Self {
        Self {
            inventory,
            model_capabilities: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_model_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = BackendModelCapabilities>,
    ) -> Self {
        self.model_capabilities = capabilities
            .into_iter()
            .map(|capability| (capability.model_id().to_string(), capability))
            .collect();
        self
    }

    #[must_use]
    pub fn route_candidate(&self) -> ExecutionRouteCandidate {
        ExecutionRouteCandidate::new("backend.ollama")
            .with_capability("llm.chat")
            .with_capability("llm.stream")
            .with_capability("runtime.ollama")
            .with_capability("agent.events.stream")
    }

    #[must_use]
    pub fn route_candidate_for_model(&self, model: &str) -> ExecutionRouteCandidate {
        let candidate = self.route_candidate();
        match self.tool_protocol_kind(model) {
            Some(ModelToolProtocolKind::NativeTools) => {
                candidate.with_capability("agent.protocol.native_tool_calls")
            }
            Some(ModelToolProtocolKind::ConstrainedJson) => {
                candidate.with_capability("agent.protocol.constrained_json")
            }
            None => candidate,
        }
    }

    #[must_use]
    pub fn chat_payload(prompt: &BackendPrompt) -> Value {
        let mut payload = json!({
            "model":prompt.model(),
            "messages":prompt.ollama_messages(),
            "stream":false
        });
        if !prompt.tools().is_empty() {
            payload["tools"] = json!(provider_tools(prompt.tools()));
        }
        payload
    }

    fn execution_payload(&self, prompt: &BackendPrompt) -> Value {
        let mut payload = Self::chat_payload(prompt);
        if let Some(context_window) = prompt.context_window_tokens() {
            let model_max = self
                .model_capabilities
                .get(prompt.model())
                .and_then(BackendModelCapabilities::context_window)
                .unwrap_or(u64::from(context_window));
            payload["options"]["num_ctx"] = json!(u64::from(context_window).min(model_max));
        }
        payload
    }

    pub fn execute_chat(&self, endpoint: &str, prompt: &BackendPrompt) -> Result<String, String> {
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        self.require_prompt_capabilities(prompt)?;
        let url = ollama_chat_url(endpoint);
        let mut payload = self.execution_payload(prompt);
        if self.tool_protocol_kind(prompt.model()) == Some(ModelToolProtocolKind::ConstrainedJson) {
            payload["format"] = json!("json");
        }
        let response = request_client(prompt)?
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|error| format!("ollama_request_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!("ollama_http_status:{}", response.status()));
        }
        let value = response
            .json::<Value>()
            .map_err(|error| format!("ollama_response_json:{error}"))?;
        self.response_to_agent_text(prompt, &url, &value)
    }

    pub fn execute_chat_stream(
        &self,
        endpoint: &str,
        prompt: &BackendPrompt,
        cancellation: &AtomicBool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String, String> {
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        self.require_prompt_capabilities(prompt)?;
        let url = ollama_chat_url(endpoint);
        let mut payload = self.execution_payload(prompt);
        payload["stream"] = json!(true);
        if self.tool_protocol_kind(prompt.model()) == Some(ModelToolProtocolKind::ConstrainedJson) {
            payload["format"] = json!("json");
        }
        let client = request_client(prompt)?;
        let value =
            crate::ollama_stream::execute(&client, &url, payload, cancellation, &mut on_delta)?;
        self.response_to_agent_text(prompt, &url, &value)
    }

    #[must_use]
    pub fn execute(&self, prompt: BackendPrompt) -> BackendExecutionResult {
        if !self.inventory.contains(prompt.model()) {
            return BackendExecutionResult::blocked("model_unavailable");
        }
        if let Err(reason) = self.require_prompt_capabilities(&prompt) {
            return BackendExecutionResult::blocked(reason);
        }

        BackendExecutionResult::ready()
    }

    #[must_use]
    pub fn create_session(&self, session_id: impl Into<String>) -> AgentSession {
        AgentSession::new(session_id, "backend.ollama")
    }

    fn require_prompt_capabilities(&self, prompt: &BackendPrompt) -> Result<(), String> {
        if prompt.tools().is_empty() {
            return Ok(());
        }
        match self.agent_tool_state(prompt.model()) {
            ModelCapabilityState::Confirmed => Ok(()),
            ModelCapabilityState::Unsupported => Err("model_native_tools_unsupported".to_string()),
            ModelCapabilityState::ProbeRequired => {
                Err("model_tool_capability_unverified".to_string())
            }
        }
    }

    fn agent_tool_state(&self, model: &str) -> ModelCapabilityState {
        let Some(capabilities) = self.model_capabilities.get(model) else {
            return ModelCapabilityState::ProbeRequired;
        };
        match capabilities.capability_state("tools") {
            ModelCapabilityState::Confirmed if capabilities.tool_protocol_certified() => {
                ModelCapabilityState::Confirmed
            }
            ModelCapabilityState::Confirmed | ModelCapabilityState::ProbeRequired => {
                ModelCapabilityState::ProbeRequired
            }
            ModelCapabilityState::Unsupported => ModelCapabilityState::Unsupported,
        }
    }

    fn tool_protocol_kind(&self, model: &str) -> Option<ModelToolProtocolKind> {
        self.model_capabilities
            .get(model)
            .and_then(BackendModelCapabilities::tool_protocol_kind)
    }

    fn response_to_agent_text(
        &self,
        prompt: &BackendPrompt,
        url: &str,
        value: &Value,
    ) -> Result<String, String> {
        if prompt.tools().is_empty() {
            return value["message"]["content"]
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| "provider_response_missing_content".to_string());
        }
        if has_native_tool_call_payload(value) {
            return backend_response_to_agent_text(parse_ollama_tool_response(
                value,
                BackendToolCallEvidence::native("backend.ollama", prompt.model(), url, false),
            ));
        }
        match self.tool_protocol_kind(prompt.model()) {
            Some(ModelToolProtocolKind::NativeTools) => {
                backend_response_to_agent_text(parse_ollama_tool_response(
                    value,
                    BackendToolCallEvidence::native("backend.ollama", prompt.model(), url, false),
                ))
            }
            Some(ModelToolProtocolKind::ConstrainedJson) => value["message"]["content"]
                .as_str()
                .ok_or_else(|| "provider_response_missing_content".to_string())
                .and_then(parse_constrained_tool_text),
            None => Err("model_tool_protocol_uncertified".to_string()),
        }
    }
}

fn request_client(prompt: &BackendPrompt) -> Result<Client, String> {
    let mut builder = Client::builder();
    if let Some(seconds) = prompt.request_timeout_seconds() {
        builder = builder.timeout(Duration::from_secs(seconds));
    }
    builder
        .build()
        .map_err(|error| format!("ollama_client_build_failed:{error}"))
}

fn has_native_tool_call_payload(value: &Value) -> bool {
    match value["message"].get("tool_calls") {
        Some(Value::Array(calls)) => !calls.is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

#[must_use]
pub fn ollama_chat_url(endpoint: &str) -> String {
    format!("{}/api/chat", endpoint.trim_end_matches('/'))
}
