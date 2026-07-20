use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{
    BackendModelCapabilities, BackendToolCallEvidence, BackendToolSchema, ModelCapabilityState,
    ModelToolProtocolCertification, ModelToolProtocolKind, OllamaExecutionBackend,
    parse_constrained_tool_text, parse_ollama_tool_response,
};

const CANARY_TOOL: &str = "desktoplab.list_files";
const MAX_CACHE_ENTRIES: usize = 256;

#[derive(Clone, Debug)]
pub struct OllamaToolProtocolCanary {
    cache: Arc<Mutex<HashMap<String, ModelToolProtocolCertification>>>,
}

impl Default for OllamaToolProtocolCanary {
    fn default() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl OllamaToolProtocolCanary {
    #[must_use]
    pub fn certify(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let fingerprint = capabilities.fingerprint();
        if let Some(cached) = self.cached(fingerprint) {
            return cached;
        }
        let result = if capabilities.capability_state("tools") != ModelCapabilityState::Confirmed {
            ModelToolProtocolCertification::failed(fingerprint, "model_tools_not_confirmed")
        } else {
            self.run_canary(endpoint, capabilities, request_timeout_seconds)
        };
        self.insert_cache(fingerprint.to_string(), result.clone());
        result
    }

    #[must_use]
    pub fn certify_fresh(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let result = if capabilities.capability_state("tools") != ModelCapabilityState::Confirmed {
            ModelToolProtocolCertification::failed(
                capabilities.fingerprint(),
                "model_tools_not_confirmed",
            )
        } else {
            self.run_canary(endpoint, capabilities, request_timeout_seconds)
        };
        self.insert_cache(capabilities.fingerprint().to_string(), result.clone());
        result
    }

    fn run_canary(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let fingerprint = capabilities.fingerprint();
        let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));
        let prompt = crate::BackendPrompt::new(
            capabilities.model_id(),
            "Call desktoplab.list_files exactly once with path set to '.'. Do not answer in prose.",
        )
        .with_tools(vec![canary_schema()]);
        let mut payload = OllamaExecutionBackend::chat_payload(&prompt);
        payload["think"] = json!(false);
        payload["options"] = json!({"temperature":0,"num_predict":128});
        let client = match Client::builder()
            .timeout(std::time::Duration::from_secs(request_timeout_seconds))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                return ModelToolProtocolCertification::failed(
                    fingerprint,
                    format!("ollama_canary_client_build_failed:{error}"),
                );
            }
        };
        let response = match client.post(&url).json(&payload).send() {
            Ok(response) if response.status().is_success() => response,
            Ok(response) => {
                return ModelToolProtocolCertification::failed(
                    fingerprint,
                    format!("ollama_canary_http_status:{}", response.status()),
                );
            }
            Err(error) => {
                return ModelToolProtocolCertification::failed(
                    fingerprint,
                    format!("ollama_canary_request_failed:{error}"),
                );
            }
        };
        let raw = match response.json::<Value>() {
            Ok(value) => value,
            Err(error) => {
                return ModelToolProtocolCertification::failed(
                    fingerprint,
                    format!("ollama_canary_response_json:{error}"),
                );
            }
        };
        validate_canary_response(fingerprint, &url, capabilities.model_id(), &raw)
    }

    fn cached(&self, fingerprint: &str) -> Option<ModelToolProtocolCertification> {
        self.cache.lock().ok()?.get(fingerprint).cloned()
    }

    fn insert_cache(&self, key: String, value: ModelToolProtocolCertification) {
        let Ok(mut cache) = self.cache.lock() else {
            return;
        };
        if cache.len() >= MAX_CACHE_ENTRIES
            && let Some(oldest) = cache.keys().next().cloned()
        {
            cache.remove(&oldest);
        }
        cache.insert(key, value);
    }
}

fn validate_canary_response(
    fingerprint: &str,
    url: &str,
    model_id: &str,
    raw: &Value,
) -> ModelToolProtocolCertification {
    let parsed = parse_ollama_tool_response(
        raw,
        BackendToolCallEvidence::native("backend.ollama", model_id, url, false),
    );
    if let Some(error) = parsed.protocol_error() {
        return ModelToolProtocolCertification::failed(fingerprint, error);
    }
    if let [call] = parsed.tool_calls() {
        if call.name() == CANARY_TOOL && call.arguments()["path"] == "." {
            return ModelToolProtocolCertification::certified_as(
                fingerprint,
                ModelToolProtocolKind::NativeTools,
            );
        }
        return ModelToolProtocolCertification::failed(
            fingerprint,
            "ollama_canary_contract_mismatch",
        );
    }
    let Some(content) = raw["message"]["content"].as_str() else {
        return ModelToolProtocolCertification::failed(
            fingerprint,
            "ollama_canary_expected_one_tool_call",
        );
    };
    let normalized = match parse_constrained_tool_text(content) {
        Ok(normalized) => normalized,
        Err(error) => return ModelToolProtocolCertification::failed(fingerprint, error),
    };
    let normalized = serde_json::from_str::<Value>(&normalized).expect("normalized JSON is valid");
    if normalized["tool"] != CANARY_TOOL || normalized["arguments"]["path"] != "." {
        return ModelToolProtocolCertification::failed(
            fingerprint,
            "ollama_canary_contract_mismatch",
        );
    }
    ModelToolProtocolCertification::certified_as(
        fingerprint,
        ModelToolProtocolKind::ConstrainedJson,
    )
}

fn canary_schema() -> BackendToolSchema {
    BackendToolSchema::new(
        CANARY_TOOL,
        "List files inside the current DesktopLab workspace.",
        json!({
            "type":"object",
            "properties":{"path":{"type":"string"}},
            "required":["path"]
        }),
    )
}
