use serde_json::{Value, json};

pub(crate) fn backend_display_name(backend_id: Option<&str>) -> Value {
    match backend_id {
        Some("backend.ollama") => json!("Local runner"),
        Some("backend.lm-studio") => json!("LM Studio"),
        Some("backend.high-end-local") => json!("High-capacity local runner"),
        Some("backend.codex") => json!("Codex bridge"),
        Some(other) => json!(other),
        None => Value::Null,
    }
}

pub(crate) fn backend_kind(backend_id: Option<&str>) -> Value {
    match backend_id {
        Some("backend.ollama") => json!("local"),
        Some("backend.lm-studio") => json!("local"),
        Some("backend.high-end-local") => json!("local"),
        Some("backend.codex") => json!("external"),
        Some(_) => json!("custom"),
        None => Value::Null,
    }
}

pub(crate) fn model_display_name(backend_id: Option<&str>, local_model: Option<&str>) -> Value {
    match backend_id {
        Some("backend.ollama") => json!(local_model.unwrap_or("No local model selected")),
        Some("backend.lm-studio") => json!(local_model.unwrap_or("Local model")),
        Some("backend.high-end-local") => json!(local_model.unwrap_or("Local model")),
        Some(_) | None => Value::Null,
    }
}

pub(crate) fn runtime_display_name(backend_id: Option<&str>, local_runtime: Option<&str>) -> Value {
    match backend_id {
        Some("backend.ollama") => json!(local_runtime.unwrap_or("Ollama")),
        Some("backend.lm-studio") => json!(local_runtime.unwrap_or("LM Studio")),
        Some("backend.high-end-local") => json!(local_runtime.unwrap_or("High-capacity runner")),
        Some("backend.codex") => json!("Codex"),
        Some(_) | None => Value::Null,
    }
}

pub(crate) fn local_model_display_name(model_id: Option<&str>) -> Option<String> {
    let model_id = model_id?;
    desktoplab_model_manager::ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == model_id)
        .map(|variant| {
            format!(
                "{} {}B {}",
                variant.family_name(),
                variant.parameters_billion(),
                variant.quantization()
            )
        })
}

pub(crate) fn local_runtime_display_name(runtime_id: Option<&str>) -> Option<String> {
    match runtime_id? {
        "runtime.ollama" => Some("Ollama".to_string()),
        "runtime.lm-studio" => Some("LM Studio".to_string()),
        "runtime.nim" => Some("NVIDIA NIM".to_string()),
        "runtime.tensorrt-llm" => Some("TensorRT-LLM".to_string()),
        "runtime.vllm" => Some("vLLM".to_string()),
        "runtime.llama-cpp-server" => Some("llama.cpp server".to_string()),
        "runtime.openai-compatible-local" => Some("Local model service".to_string()),
        "runtime.custom-lan" => Some("Private network model service".to_string()),
        other => Some(other.to_string()),
    }
}
