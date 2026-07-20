use serde_json::Value;

pub(crate) struct SetupAcceptSelection {
    pub(crate) runtime_id: Option<String>,
    pub(crate) model_id: Option<String>,
}

pub(crate) fn setup_accept_selection(body: &str) -> SetupAcceptSelection {
    let parsed = serde_json::from_str::<Value>(body).ok();
    SetupAcceptSelection {
        runtime_id: required_string(&parsed, "runtimeId"),
        model_id: required_string(&parsed, "modelId"),
    }
}

pub(crate) fn valid_runtime_id(runtime_id: &str) -> bool {
    matches!(runtime_id, "runtime.ollama" | "runtime.lm-studio")
        || (runtime_id == "runtime.mlx-lm"
            && cfg!(all(target_os = "macos", target_arch = "aarch64")))
}

pub(crate) fn valid_model_for_runtime(model_id: &str, runtime_id: &str) -> bool {
    desktoplab_model_manager::ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .any(|variant| {
            variant.model_id() == model_id
                && variant.runtime_compatibility().runtime_id() == runtime_id
        })
}

fn required_string(parsed: &Option<Value>, field: &str) -> Option<String> {
    parsed
        .as_ref()
        .and_then(|value| value.get(field).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(String::from)
}
