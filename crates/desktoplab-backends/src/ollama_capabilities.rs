use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::BackendModelCapabilities;

const MAX_CACHE_ENTRIES: usize = 256;

#[derive(Clone, Debug)]
pub struct OllamaModelCapabilityResolver {
    client: Client,
    cache: Arc<Mutex<HashMap<String, BackendModelCapabilities>>>,
}

impl Default for OllamaModelCapabilityResolver {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Ollama capability client should build"),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl OllamaModelCapabilityResolver {
    pub fn resolve(
        &self,
        endpoint: &str,
        requested_model: &str,
    ) -> Result<BackendModelCapabilities, String> {
        let endpoint = endpoint.trim_end_matches('/');
        let tags = self
            .client
            .get(format!("{endpoint}/api/tags"))
            .send()
            .map_err(|error| format!("ollama_capability_tags_failed:{error}"))?;
        if !tags.status().is_success() {
            return Err(format!("ollama_capability_tags_status:{}", tags.status()));
        }
        let tags = tags
            .json::<Value>()
            .map_err(|error| format!("ollama_capability_tags_json:{error}"))?;
        let model = find_model(&tags, requested_model).ok_or("model_unavailable")?;
        let model_name = model["name"].as_str().ok_or("model_unavailable")?;
        let version = model["digest"]
            .as_str()
            .or_else(|| model["modified_at"].as_str())
            .map(ToString::to_string);
        let cache_key = version
            .as_deref()
            .map(|version| format!("{endpoint}|{model_name}|{version}"));
        if let Some(cached) = cache_key.as_deref().and_then(|key| self.cached(key)) {
            return Ok(cached);
        }

        let show = self
            .client
            .post(format!("{endpoint}/api/show"))
            .json(&json!({"name":model_name}))
            .send()
            .map_err(|error| format!("ollama_capability_show_failed:{error}"))?;
        if !show.status().is_success() {
            return Err(format!("ollama_capability_show_status:{}", show.status()));
        }
        let show = show
            .json::<Value>()
            .map_err(|error| format!("ollama_capability_show_json:{error}"))?;
        let context_window = context_window(&show);
        let capabilities = show.get("capabilities").and_then(Value::as_array);
        let result = if let Some(capabilities) = capabilities {
            BackendModelCapabilities::reported(
                "backend.ollama",
                model_name,
                version,
                context_window,
                capabilities.iter().filter_map(Value::as_str),
            )
        } else {
            BackendModelCapabilities::unverified(
                "backend.ollama",
                model_name,
                version,
                context_window,
            )
        };
        if let Some(key) = cache_key {
            self.insert_cache(key, result.clone());
        }
        Ok(result)
    }

    fn cached(&self, key: &str) -> Option<BackendModelCapabilities> {
        self.cache
            .lock()
            .expect("Ollama capability cache should not be poisoned")
            .get(key)
            .cloned()
    }

    fn insert_cache(&self, key: String, value: BackendModelCapabilities) {
        let mut cache = self
            .cache
            .lock()
            .expect("Ollama capability cache should not be poisoned");
        if cache.len() >= MAX_CACHE_ENTRIES
            && let Some(oldest) = cache.keys().next().cloned()
        {
            cache.remove(&oldest);
        }
        cache.insert(key, value);
    }
}

fn find_model<'a>(tags: &'a Value, requested_model: &str) -> Option<&'a Value> {
    tags.get("models")?
        .as_array()?
        .iter()
        .find(|model| model["name"].as_str() == Some(requested_model))
}

fn context_window(show: &Value) -> Option<u64> {
    let model_context = show
        .get("model_info")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|info| info.iter())
        .find_map(|(key, value)| {
            key.ends_with(".context_length")
                .then(|| value.as_u64())
                .flatten()
        });
    let parameter_context = show
        .get("parameters")
        .and_then(Value::as_str)
        .and_then(parse_num_ctx);
    match (model_context, parameter_context) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

fn parse_num_ctx(parameters: &str) -> Option<u64> {
    parameters
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next() == Some("num_ctx"))
                .then(|| parts.next()?.parse::<u64>().ok())
                .flatten()
                .filter(|value| *value > 0)
        })
        .next_back()
}
