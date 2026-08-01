use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use crate::{
    BackendModelCapabilities, BackendToolCallEvidence, ModelCapabilityState,
    ModelProtocolCertificationState, ModelToolProtocolCertification, ModelToolProtocolKind,
    parse_constrained_tool_text, parse_openai_compatible_tool_response,
};

const CANARY_TOOL: &str = "desktoplab.list_files";
const MAX_CACHE_ENTRIES: usize = 256;

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleToolProtocolCanary {
    cache: Arc<Mutex<HashMap<String, ModelToolProtocolCertification>>>,
    #[cfg(debug_assertions)]
    response_for_test: Option<Value>,
}

impl Default for OpenAiCompatibleToolProtocolCanary {
    fn default() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(debug_assertions)]
            response_for_test: None,
        }
    }
}

impl OpenAiCompatibleToolProtocolCanary {
    #[cfg(debug_assertions)]
    #[must_use]
    pub fn with_response_for_test(response: Value) -> Self {
        Self {
            response_for_test: Some(response),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn certify(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        protocol: ModelToolProtocolKind,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let fingerprint = capabilities.fingerprint();
        if let Some(cached) = self.cached(fingerprint) {
            return cached;
        }
        let result = if capabilities.capability_state("tools") != ModelCapabilityState::Confirmed {
            ModelToolProtocolCertification::failed(fingerprint, "model_tools_not_confirmed")
        } else if !loopback_endpoint(endpoint) {
            ModelToolProtocolCertification::failed(
                fingerprint,
                "local_canary_endpoint_not_loopback",
            )
        } else {
            self.run_canary(endpoint, capabilities, protocol, request_timeout_seconds)
        };
        if result.state() == ModelProtocolCertificationState::Certified {
            self.insert_cache(fingerprint.to_string(), result.clone());
        }
        result
    }

    #[must_use]
    pub fn certify_fresh(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        protocol: ModelToolProtocolKind,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let fingerprint = capabilities.fingerprint();
        let result = if capabilities.capability_state("tools") != ModelCapabilityState::Confirmed {
            ModelToolProtocolCertification::failed(fingerprint, "model_tools_not_confirmed")
        } else if !loopback_endpoint(endpoint) {
            ModelToolProtocolCertification::failed(
                fingerprint,
                "local_canary_endpoint_not_loopback",
            )
        } else {
            self.run_canary(endpoint, capabilities, protocol, request_timeout_seconds)
        };
        if result.state() == ModelProtocolCertificationState::Certified {
            self.insert_cache(fingerprint.to_string(), result.clone());
        }
        result
    }

    fn run_canary(
        &self,
        endpoint: &str,
        capabilities: &BackendModelCapabilities,
        protocol: ModelToolProtocolKind,
        request_timeout_seconds: u64,
    ) -> ModelToolProtocolCertification {
        let fingerprint = capabilities.fingerprint();
        let url = format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'));
        let payload = canary_payload(capabilities.model_id(), protocol);
        let raw = match self.response(&url, &payload, request_timeout_seconds) {
            Ok(raw) => raw,
            Err(error) => return ModelToolProtocolCertification::failed(fingerprint, error),
        };
        validate_response(
            fingerprint,
            capabilities.backend_id(),
            capabilities.model_id(),
            &url,
            protocol,
            &raw,
        )
    }

    fn response(
        &self,
        url: &str,
        payload: &Value,
        request_timeout_seconds: u64,
    ) -> Result<Value, String> {
        #[cfg(debug_assertions)]
        if let Some(response) = &self.response_for_test {
            return Ok(response.clone());
        }
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(request_timeout_seconds))
            .build()
            .map_err(|error| format!("local_canary_client_build_failed:{error}"))?;
        let response = client
            .post(url)
            .json(payload)
            .send()
            .map_err(|error| format!("local_canary_request_failed:{error}"))?;
        if !response.status().is_success() {
            return Err(format!("local_canary_http_status:{}", response.status()));
        }
        response
            .json::<Value>()
            .map_err(|error| format!("local_canary_response_json:{error}"))
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

fn canary_payload(model: &str, protocol: ModelToolProtocolKind) -> Value {
    let prompt = match protocol {
        ModelToolProtocolKind::NativeTools => {
            "Call desktoplab.list_files exactly once with path set to '.'. Do not answer in prose."
        }
        ModelToolProtocolKind::ConstrainedJson => {
            "Return only this tool call as JSON: {\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"
        }
    };
    let mut payload = json!({
        "model":model,
        "messages":[{"role":"user","content":prompt}],
        "stream":false,
        "temperature":0,
        "max_tokens":512
    });
    if protocol == ModelToolProtocolKind::NativeTools {
        payload["tools"] = json!([{
            "type":"function",
            "function":{
                "name":CANARY_TOOL,
                "description":"List files inside the current DesktopLab workspace.",
                "parameters":{
                    "type":"object",
                    "properties":{"path":{"type":"string"}},
                    "required":["path"]
                }
            }
        }]);
        payload["tool_choice"] = json!("required");
    }
    payload
}

fn validate_response(
    fingerprint: &str,
    backend_id: &str,
    model_id: &str,
    url: &str,
    protocol: ModelToolProtocolKind,
    raw: &Value,
) -> ModelToolProtocolCertification {
    let matches = match protocol {
        ModelToolProtocolKind::NativeTools => native_call_matches(backend_id, model_id, url, raw),
        ModelToolProtocolKind::ConstrainedJson => constrained_call_matches(raw),
    };
    match matches {
        Ok(true) => ModelToolProtocolCertification::certified_as(fingerprint, protocol),
        Ok(false) => {
            ModelToolProtocolCertification::failed(fingerprint, "local_canary_contract_mismatch")
        }
        Err(error) => ModelToolProtocolCertification::failed(fingerprint, error),
    }
}

fn native_call_matches(
    backend_id: &str,
    model_id: &str,
    url: &str,
    raw: &Value,
) -> Result<bool, String> {
    let parsed = parse_openai_compatible_tool_response(
        raw,
        BackendToolCallEvidence::native(backend_id, model_id, url, false),
    );
    if let Some(error) = parsed.protocol_error() {
        return Err(error.to_string());
    }
    Ok(
        matches!(parsed.tool_calls(), [call] if call.name() == CANARY_TOOL && call.arguments()["path"] == "."),
    )
}

fn constrained_call_matches(raw: &Value) -> Result<bool, String> {
    let content = raw["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "local_canary_expected_constrained_content".to_string())?;
    let normalized = parse_constrained_tool_text(content)?;
    let value = serde_json::from_str::<Value>(&normalized)
        .map_err(|_| "local_canary_normalized_json_invalid".to_string())?;
    Ok(value["tool"] == CANARY_TOOL && value["arguments"]["path"] == ".")
}

fn loopback_endpoint(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint.trim()) else {
        return false;
    };
    if url.scheme() != "http" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}
