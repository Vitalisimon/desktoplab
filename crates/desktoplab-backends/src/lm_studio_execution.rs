use desktoplab_execution_router::ExecutionRouteCandidate;
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

use crate::{
    BackendExecutionResult, BackendModelInventory, BackendPrompt, BackendToolCallEvidence,
    ProviderCompatibilityProfile, backend_response_to_agent_text, parse_constrained_tool_text,
    parse_openai_compatible_tool_response, provider_tools,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEndpoint {
    url: String,
    available: bool,
    reason: Option<String>,
}

impl LocalEndpoint {
    #[must_use]
    pub fn available(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            available: true,
            reason: None,
        }
    }

    #[must_use]
    pub fn unavailable(url: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            available: false,
            reason: Some(reason.into()),
        }
    }

    pub(crate) fn chat_completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.url.trim_end_matches('/'))
    }

    pub(crate) fn is_available(&self) -> bool {
        self.available
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioExecutionBackend {
    endpoint: LocalEndpoint,
    inventory: BackendModelInventory,
}

impl LmStudioExecutionBackend {
    #[must_use]
    pub fn new(endpoint: LocalEndpoint, inventory: BackendModelInventory) -> Self {
        Self {
            endpoint,
            inventory,
        }
    }

    #[must_use]
    pub fn route_candidate(&self) -> ExecutionRouteCandidate {
        let candidate = ExecutionRouteCandidate::new("backend.lm-studio")
            .with_capability("llm.chat")
            .with_capability("llm.stream")
            .with_capability("api.openai-compatible.local")
            .with_capability("runtime.lm-studio")
            .with_capability("agent.protocol.native_tool_calls");

        if self.endpoint.available {
            candidate
        } else {
            candidate.mark_unavailable(
                self.endpoint
                    .reason
                    .clone()
                    .unwrap_or_else(|| "endpoint unavailable".to_string()),
            )
        }
    }

    #[must_use]
    pub fn execute(&self, prompt: BackendPrompt) -> BackendExecutionResult {
        if !self.endpoint.available {
            return BackendExecutionResult::blocked("endpoint_unavailable");
        }
        if !self.inventory.contains(prompt.model()) {
            return BackendExecutionResult::blocked("model_unavailable");
        }
        BackendExecutionResult::ready()
    }

    #[must_use]
    pub fn chat_completion_payload(&self, prompt: &BackendPrompt) -> Value {
        let mut payload = json!({
            "model":prompt.model(),
            "messages":prompt.openai_messages(),
            "stream":false
        });
        if !prompt.tools().is_empty() {
            payload["tools"] = json!(provider_tools(prompt.tools()));
            ProviderCompatibilityProfile::openai_chat_completions().apply_tool_choice(&mut payload);
        }
        payload
    }

    #[must_use]
    pub fn constrained_chat_completion_payload(&self, prompt: &BackendPrompt) -> Value {
        let mut messages = prompt.openai_messages();
        messages.insert(
            0,
            json!({
                "role":"system",
                "content":constrained_tool_contract(prompt)
            }),
        );
        json!({
            "model":prompt.model(),
            "messages":messages,
            "stream":false
        })
    }

    pub fn execute_chat(&self, prompt: &BackendPrompt) -> Result<String, String> {
        if !self.endpoint.available {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        let url = self.chat_completions_url();
        let response = reqwest::blocking::Client::new()
            .post(&url)
            .json(&self.chat_completion_payload(prompt))
            .send()
            .map_err(|error| format!("lm_studio_request_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!("lm_studio_http_status:{}", response.status()));
        }
        let value = response
            .json::<Value>()
            .map_err(|error| format!("lm_studio_response_json:{error}"))?;
        backend_response_to_agent_text(parse_openai_compatible_tool_response(
            &value,
            BackendToolCallEvidence::native("backend.lm-studio", prompt.model(), &url, false),
        ))
    }

    pub fn execute_chat_stream(
        &self,
        prompt: &BackendPrompt,
        cancellation: &AtomicBool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String, String> {
        if !self.endpoint.available {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        let url = self.chat_completions_url();
        let mut payload = self.chat_completion_payload(prompt);
        payload["stream"] = json!(true);
        let value =
            crate::openai_compatible_stream::execute(&url, payload, cancellation, &mut on_delta)?;
        backend_response_to_agent_text(parse_openai_compatible_tool_response(
            &value,
            BackendToolCallEvidence::native("backend.lm-studio", prompt.model(), &url, false),
        ))
    }

    pub fn execute_constrained_chat(&self, prompt: &BackendPrompt) -> Result<String, String> {
        if !self.endpoint.available {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        let response = reqwest::blocking::Client::new()
            .post(self.chat_completions_url())
            .json(&self.constrained_chat_completion_payload(prompt))
            .send()
            .map_err(|error| format!("mlx_request_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!("mlx_http_status:{}", response.status()));
        }
        let value = response
            .json::<Value>()
            .map_err(|error| format!("mlx_response_json:{error}"))?;
        constrained_response_to_agent_text(&value)
    }

    pub fn execute_constrained_chat_stream(
        &self,
        prompt: &BackendPrompt,
        cancellation: &AtomicBool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String, String> {
        if !self.endpoint.available {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        let url = self.chat_completions_url();
        let mut payload = self.constrained_chat_completion_payload(prompt);
        payload["stream"] = json!(true);
        let value =
            crate::openai_compatible_stream::execute(&url, payload, cancellation, &mut on_delta)?;
        constrained_response_to_agent_text(&value)
    }

    #[must_use]
    pub fn chat_completions_url(&self) -> String {
        self.endpoint.chat_completions_url()
    }

    #[must_use]
    pub fn requires_provider_credential(&self) -> bool {
        false
    }
}

fn constrained_tool_contract(prompt: &BackendPrompt) -> String {
    format!(
        "Return exactly one JSON object and no prose or Markdown. The object must use this shape: {contract}. Select one exact tool name from the schemas and provide an arguments object satisfying that schema. Tool schemas: {tools}",
        contract = r#"{"name":"desktoplab.tool_name","arguments":{}}"#,
        tools = Value::Array(provider_tools(prompt.tools()))
    )
}

fn constrained_response_to_agent_text(value: &Value) -> Result<String, String> {
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "provider_response_missing_content".to_string())?;
    parse_constrained_tool_text(content)
}
