use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};

use crate::{
    BackendModelInventory, BackendPrompt, BackendToolCallEvidence, LocalEndpoint,
    ProviderCompatibilityProfile, backend_response_to_agent_text,
    parse_openai_compatible_tool_response, provider_tools,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCompatibleLocalExecutionBackend {
    backend_id: String,
    endpoint: LocalEndpoint,
    inventory: BackendModelInventory,
}

impl OpenAiCompatibleLocalExecutionBackend {
    #[must_use]
    pub fn new(
        backend_id: impl Into<String>,
        endpoint: LocalEndpoint,
        inventory: BackendModelInventory,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            endpoint,
            inventory,
        }
    }

    pub fn execute_chat(&self, prompt: &BackendPrompt) -> Result<String, String> {
        if !self.endpoint.is_available() {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        let url = self.endpoint.chat_completions_url();
        let response = reqwest::blocking::Client::new()
            .post(&url)
            .json(&chat_completion_payload(prompt))
            .send()
            .map_err(|error| format!("openai_compatible_local_request_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "openai_compatible_local_http_status:{}",
                response.status()
            ));
        }
        let value = response
            .json::<Value>()
            .map_err(|error| format!("openai_compatible_local_response_json:{error}"))?;
        backend_response_to_agent_text(parse_openai_compatible_tool_response(
            &value,
            BackendToolCallEvidence::native(&self.backend_id, prompt.model(), &url, false),
        ))
    }

    pub fn execute_chat_stream(
        &self,
        prompt: &BackendPrompt,
        cancellation: &AtomicBool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String, String> {
        self.require_ready(prompt)?;
        let url = self.endpoint.chat_completions_url();
        let mut payload = chat_completion_payload(prompt);
        payload["stream"] = json!(true);
        let value =
            crate::openai_compatible_stream::execute(&url, payload, cancellation, &mut on_delta)?;
        backend_response_to_agent_text(parse_openai_compatible_tool_response(
            &value,
            BackendToolCallEvidence::native(&self.backend_id, prompt.model(), &url, false),
        ))
    }

    fn require_ready(&self, prompt: &BackendPrompt) -> Result<(), String> {
        if !self.endpoint.is_available() {
            return Err("endpoint_unavailable".to_string());
        }
        if !self.inventory.contains(prompt.model()) {
            return Err("model_unavailable".to_string());
        }
        Ok(())
    }
}

fn chat_completion_payload(prompt: &BackendPrompt) -> Value {
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
