use serde_json::{Value, json};

use crate::execution_route_labels::{local_model_display_name, local_runtime_display_name};
use desktoplab_runtime::{HighEndRuntimeHealthState, HighEndRuntimeLifecycle};

#[must_use]
pub fn route_options_response_from_inventory(
    selected_route_id: &str,
    local_route_ready: bool,
    local_disabled_reason: Option<&str>,
    runtime_id: Option<&str>,
    model_id: Option<&str>,
    codex_bridge_ready: bool,
    inventory: &Value,
    high_end_runtime: Option<&HighEndRuntimeLifecycle>,
) -> Value {
    let model =
        local_model_display_name(model_id).unwrap_or_else(|| "No local model selected".to_string());
    let runtime = local_runtime_display_name(runtime_id).unwrap_or_else(|| "Ollama".to_string());
    let selected_local_route_id = model_id.map_or_else(
        || selected_route_id.to_string(),
        crate::execution_routes::local_route_id,
    );
    let mut options = local_options_from_inventory(
        inventory,
        &selected_local_route_id,
        local_route_ready,
        local_disabled_reason,
    );
    prioritize_selected_local_route(&mut options, &selected_local_route_id);
    if options.is_empty() {
        let backend_id = local_backend_id(runtime_id.unwrap_or("runtime.ollama"));
        let disabled_reason =
            local_disabled_reason.unwrap_or("Choose and validate a local agent model.");
        options.push(json!({
            "routeId":selected_local_route_id,
            "backendId":backend_id,
            "backendKind":"local",
            "label":format!("{model} · {runtime}"),
            "modelId":model_id,
            "runtimeId":runtime_id,
            "executionBackendId":backend_id,
            "modelDisplayName":model,
            "runtimeDisplayName":runtime,
            "status":"unavailable",
            "disabledReason":disabled_reason
        }));
    }
    if let Some(runtime) = high_end_runtime
        .filter(|runtime| runtime.evidence().state() == HighEndRuntimeHealthState::ModelReady)
    {
        let runtime_id = runtime.contract().runtime_id().as_str();
        let model_id = runtime.endpoint().model_id();
        let runtime_name = local_runtime_display_name(Some(runtime_id))
            .unwrap_or_else(|| "High-capacity runner".to_string());
        options.push(json!({
            "routeId":"route.high-end-local",
            "backendId":"backend.high-end-local",
            "backendKind":"local",
            "label":format!("{model_id} · {runtime_name}"),
            "modelId":model_id,
            "runtimeId":runtime_id,
            "executionBackendId":"backend.high-end-local",
            "modelDisplayName":model_id,
            "runtimeDisplayName":runtime_name,
            "status":"unavailable",
            "disabledReason":"Agent protocol verification is required before this runner can execute work."
        }));
    }
    options.push(json!({
        "routeId":"route.cloud.openai",
        "backendId":"backend.openai",
        "backendKind":"cloud",
        "label":"OpenAI GPT-4.1 · Cloud",
        "modelDisplayName":"OpenAI GPT-4.1",
        "runtimeDisplayName":"OpenAI",
        "status":"unavailable",
        "disabledReason":"Connect OpenAI before routing work to the cloud."
    }));
    options.push(json!({
        "routeId":"route.external.codex",
        "backendId":"backend.codex",
        "backendKind":"external",
        "label":"Codex bridge · External",
        "modelDisplayName":"Codex bridge",
        "runtimeDisplayName":"Codex",
        "status":if codex_bridge_ready {"available"} else {"unavailable"},
        "disabledReason":if codex_bridge_ready {Value::Null} else {json!("Connect the Codex bridge before routing work outside DesktopLab.")},
        "egressPolicy":"requires_approval",
        "repositoryContextEgress":"approval_required"
    }));
    json!({
        "selectedRouteId":selected_route_id,
        "options":options
    })
}

fn local_options_from_inventory(
    inventory: &Value,
    selected_route_id: &str,
    selected_route_ready: bool,
    selected_disabled_reason: Option<&str>,
) -> Vec<Value> {
    let Some(models) = inventory["models"].as_array() else {
        return Vec::new();
    };
    models
        .iter()
        .filter(|model| model["backendSelectable"].as_bool().unwrap_or(false))
        .filter_map(|model| {
            let model_id = model["modelId"].as_str()?;
            Some(local_option_from_model(
                model,
                model_id,
                selected_route_id,
                selected_route_ready,
                selected_disabled_reason,
            ))
        })
        .collect()
}

fn local_option_from_model(
    model: &Value,
    model_id: &str,
    selected_route_id: &str,
    selected_route_ready: bool,
    selected_disabled_reason: Option<&str>,
) -> Value {
    let runtime_id = model["runtimeId"].as_str().unwrap_or("runtime.ollama");
    let model_display = local_model_display_name(Some(model_id)).unwrap_or_else(|| {
        model["displayName"]
            .as_str()
            .unwrap_or("Local model")
            .to_string()
    });
    let runtime_display =
        local_runtime_display_name(Some(runtime_id)).unwrap_or_else(|| runtime_id.to_string());
    let backend_id = local_backend_id(runtime_id);
    let route_id = crate::execution_routes::local_route_id(model_id);
    let selected = route_id == selected_route_id;
    json!({
        "routeId":route_id,
        "backendId":backend_id,
        "backendKind":"local",
        "label":format!("{model_display} · {runtime_display}"),
        "modelId":model_id,
        "runtimeId":runtime_id,
        "executionBackendId":backend_id,
        "modelDisplayName":model_display,
        "runtimeDisplayName":runtime_display,
        "status":if selected && !selected_route_ready {"unavailable"} else {"available"},
        "disabledReason":if selected && !selected_route_ready {
            selected_disabled_reason.map_or(Value::Null, |reason| json!(reason))
        } else {
            Value::Null
        }
    })
}

fn local_backend_id(runtime_id: &str) -> &'static str {
    match runtime_id {
        "runtime.lm-studio" => "backend.lm-studio",
        "runtime.mlx-lm" => "backend.mlx-lm",
        runtime_id if crate::high_end_runtime_routes::is_high_end_runtime_id(runtime_id) => {
            "backend.high-end-local"
        }
        _ => "backend.ollama",
    }
}

fn prioritize_selected_local_route(options: &mut [Value], selected_route_id: &str) {
    let Some(index) = options
        .iter()
        .position(|option| option["routeId"].as_str() == Some(selected_route_id))
    else {
        return;
    };
    options.rotate_left(index);
}
