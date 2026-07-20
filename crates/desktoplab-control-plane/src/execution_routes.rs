use desktoplab_acp_plugin::{AcpCapabilityStatus, acp_capability_matrix};
use desktoplab_backend_services::{
    BackendRouteCandidate, BackendRouteDecision, BackendRouteService, BackendRouteStatus,
    RouteApiPolicy, RouteApiRequest,
};
use desktoplab_backends::BackendModelCapabilities;
use desktoplab_runtime::HighEndRuntimeLifecycle;
use serde_json::{Value, json};

use crate::execution_backend_capabilities::{backend_capability_profile, backend_support_contract};
use crate::execution_external_routes::codex_route_response;
pub use crate::execution_external_routes::{
    external_agent_bridge_v2_contract, external_backends_response,
};
use crate::execution_route_labels::{
    backend_display_name, backend_kind, local_model_display_name, local_runtime_display_name,
    model_display_name, runtime_display_name,
};
use crate::execution_tool_calling_evidence::tool_calling_evidence;

pub(crate) const UNCONFIGURED_LOCAL_ROUTE_ID: &str = "route.local.unconfigured";

#[must_use]
#[allow(dead_code)]
pub fn route_response(path: &str, body: &str) -> Value {
    route_response_for_selection(UNCONFIGURED_LOCAL_ROUTE_ID, path, body)
}

#[must_use]
#[allow(dead_code)]
pub fn route_response_for_selection(route_id: &str, path: &str, body: &str) -> Value {
    route_response_for_selection_with_readiness(
        route_id, path, body, true, None, None, None, None, false,
    )
}

#[must_use]
pub fn route_response_for_selection_with_readiness(
    route_id: &str,
    path: &str,
    body: &str,
    local_route_ready: bool,
    readiness_blocked_reason: Option<&str>,
    runtime_id: Option<&str>,
    model_id: Option<&str>,
    model_capabilities: Option<&BackendModelCapabilities>,
    codex_bridge_ready: bool,
) -> Value {
    if route_id == "route.external.codex" {
        return codex_route_response(codex_bridge_ready);
    }
    let selected_model_id =
        local_model_id_from_route_id(route_id).or_else(|| model_id.map(ToString::to_string));
    let selected_runtime_id = selected_model_id
        .as_deref()
        .and_then(local_runtime_id_for_model)
        .or_else(|| runtime_id.map(ToString::to_string));
    let local_backend_id =
        local_backend_id(selected_runtime_id.as_deref().unwrap_or("runtime.ollama"));
    let capability_profile = backend_capability_profile(local_backend_id);
    let tool_calling = tool_calling_evidence(
        local_backend_id,
        selected_model_id.as_deref(),
        model_capabilities,
    );
    let full_agent_eligible = tool_calling["fullCodingAgentEligible"] == true;
    let mut backend_capabilities = capability_profile
        .advertised()
        .iter()
        .copied()
        .filter(|capability| full_agent_eligible || !capability.starts_with("tools."))
        .filter(|capability| {
            full_agent_eligible || *capability != "agent.protocol.native_tool_calls"
        })
        .filter(|capability| {
            *capability != "agent.protocol.native_tool_calls"
                || tool_calling["nativeToolCalls"] == true
        })
        .collect::<Vec<_>>();
    if tool_calling["structuredOutputSupported"] == true
        && !backend_capabilities.contains(&"agent.protocol.constrained_json")
    {
        backend_capabilities.push("agent.protocol.constrained_json");
    }
    let decision = local_route_decision(
        path,
        body,
        local_route_ready,
        readiness_blocked_reason,
        selected_runtime_id.as_deref(),
        selected_model_id.as_deref(),
    );
    let blocked_reasons = blocked_reasons_with_readiness(&decision, readiness_blocked_reason);
    let model = local_model_display_name(selected_model_id.as_deref());
    let runtime = local_runtime_display_name(selected_runtime_id.as_deref());
    json!({
        "source":"service_backed",
        "routeId":route_id,
        "status":status_value(decision.status()),
        "backendId":decision.backend_id(),
        "backendDisplayName":backend_display_name(decision.backend_id()),
        "backendKind":backend_kind(decision.backend_id()),
        "modelId":selected_model_id,
        "runtimeId":selected_runtime_id,
        "modelDisplayName":model_display_name(decision.backend_id(), model.as_deref()),
        "runtimeDisplayName":runtime_display_name(decision.backend_id(), runtime.as_deref()),
        "summary":route_summary(&decision),
        "reasons":decision.explanations(),
        "blockedReasons":blocked_reasons,
        "requiredCapabilities":capability_profile.required(),
        "backendCapabilities":backend_capabilities,
        "backendSupportContract":backend_support_contract(local_backend_id),
        "backendToolCalling":tool_calling,
        "modelAgentCapability":local_model_agent_capability(selected_model_id.as_deref(), full_agent_eligible),
        "needsFallbackApproval":blocked_reasons.contains(&"fallback_requires_visibility_or_approval".to_string())
    })
}

#[must_use]
pub fn runtime_inspect_response(
    route_id: &str,
    local_route_ready: bool,
    readiness_blocked_reason: Option<&str>,
    runtime_id: Option<&str>,
    model_id: Option<&str>,
    model_capabilities: Option<&BackendModelCapabilities>,
    codex_bridge_ready: bool,
) -> Value {
    let route = route_response_for_selection_with_readiness(
        route_id,
        "/v1/runtime/inspect",
        "",
        local_route_ready,
        readiness_blocked_reason,
        runtime_id,
        model_id,
        model_capabilities,
        codex_bridge_ready,
    );
    let configured_model_id = route["modelId"]
        .as_str()
        .map(str::to_string)
        .or_else(|| local_model_id_from_route_id(route_id))
        .or_else(|| model_id.map(ToString::to_string));
    let configured_runtime_id = route["runtimeId"]
        .as_str()
        .map(str::to_string)
        .or_else(|| runtime_id.map(ToString::to_string))
        .or_else(|| {
            configured_model_id
                .as_deref()
                .and_then(local_runtime_id_for_model)
        })
        .or_else(|| Some("runtime.ollama".to_string()));
    let backend_id = route["backendId"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            local_backend_id(configured_runtime_id.as_deref().unwrap_or("runtime.ollama"))
                .to_string()
        });
    let degraded_reason = if local_route_ready {
        Value::Null
    } else {
        json!(readiness_blocked_reason.unwrap_or("runtime_and_model_not_verified"))
    };
    json!({
        "source":"service_backed",
        "inspectState":if local_route_ready {"ready"} else {"blocked"},
        "active":{
            "selectedRouteId":route_id,
            "backendId":backend_id,
            "runtimeId":configured_runtime_id,
            "modelId":configured_model_id,
            "accountMode":if route_id == "route.external.codex" {"subscription_bridge"} else {"local_runtime"},
            "egress":if route_id == "route.external.codex" {"requires_approval"} else {"local_or_approval_gated"},
            "toolCapability":"filesystem_write_requires_approval",
            "degradedReason":degraded_reason
        },
        "evidence":{
            "coldManifest":{
                "source":"route_selection",
                "runtimeId":configured_runtime_id,
                "modelId":configured_model_id
            },
            "liveRuntime":{
                "state":if local_route_ready {"verified"} else {"not_verified"},
                "evidence":if local_route_ready {json!("runtime_and_model_verified")} else {Value::Null}
            }
        },
        "backendSupportContract":backend_support_contract(&backend_id),
        "protocolAdapters":{
            "acp":{
                "protocolVersion":1,
                "transport":"library_adapter_only",
                "publicExecutionStatus":"not_registered",
                "capabilities":acp_capability_matrix().iter().map(|(operation, status)| json!({
                    "operation":operation,
                    "status":match status { AcpCapabilityStatus::Supported => "supported", AcpCapabilityStatus::Unsupported => "unsupported" }
                })).collect::<Vec<_>>()
            }
        }
    })
}

#[must_use]
pub fn high_end_runtime_health_response(runtime: Option<&HighEndRuntimeLifecycle>) -> Value {
    crate::high_end_runtime_routes::health_response(runtime)
}

fn local_route_decision(
    path: &str,
    body: &str,
    local_route_ready: bool,
    readiness_blocked_reason: Option<&str>,
    runtime_id: Option<&str>,
    model_id: Option<&str>,
) -> BackendRouteDecision {
    let service = BackendRouteService::new(RouteApiPolicy::local_only());
    let backend_id = local_backend_id(runtime_id.unwrap_or("runtime.ollama"));
    let capability_profile = backend_capability_profile(backend_id);
    let mut candidate = BackendRouteCandidate::local(backend_id, capability_profile.required())
        .with_model(model_id.unwrap_or("unconfigured"));
    if !local_route_ready {
        candidate = candidate.mark_runtime_unavailable(
            readiness_blocked_reason.unwrap_or("runtime_and_model_not_verified"),
        );
    }
    if flag(path, body, "localModelReady") == Some(false) {
        candidate = candidate.mark_model_unavailable("model not downloaded");
    }
    service.plan(
        RouteApiRequest::new(capability_profile.required()),
        vec![candidate],
    )
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

fn local_model_agent_capability(model_id: Option<&str>, full_agent_eligible: bool) -> Value {
    let model_id = model_id.unwrap_or("model.unknown");
    let (class, route_label, claim, certification) = if full_agent_eligible {
        (
            "agent_capable",
            "Local agent",
            "The installed model fingerprint passed DesktopLab's tool protocol verification.",
            "certified_current_fingerprint",
        )
    } else {
        (
            "chat_capable",
            "Local chat",
            "Not routed as a coding agent without certification evidence.",
            "agent_protocol_required",
        )
    };
    json!({
        "class":class,
        "routeLabel":route_label,
        "claim":claim,
        "certification":certification,
        "modelId":model_id
    })
}

pub fn local_route_id(model_id: &str) -> String {
    format!("route.local.{}", model_id.trim_start_matches("model."))
}

pub fn local_model_id_from_route_id(route_id: &str) -> Option<String> {
    if route_id == UNCONFIGURED_LOCAL_ROUTE_ID {
        return None;
    }
    let suffix = route_id.strip_prefix("route.local.")?;
    Some(format!("model.{suffix}"))
}

fn local_runtime_id_for_model(model_id: &str) -> Option<String> {
    desktoplab_model_manager::ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == model_id)
        .map(|variant| variant.runtime_compatibility().runtime_id().to_string())
}

fn blocked_reasons_with_readiness(
    decision: &BackendRouteDecision,
    readiness_blocked_reason: Option<&str>,
) -> Vec<String> {
    let mut reasons = decision.blocked_reasons().to_vec();
    if let Some(reason) = readiness_blocked_reason
        && !reasons.iter().any(|candidate| candidate == reason)
    {
        reasons.insert(0, reason.to_string());
    }
    reasons
}

fn status_value(status: BackendRouteStatus) -> &'static str {
    match status {
        BackendRouteStatus::Selected => "selected",
        BackendRouteStatus::Blocked => "blocked",
    }
}

fn route_summary(decision: &BackendRouteDecision) -> &'static str {
    if decision.status() == BackendRouteStatus::Selected {
        "Runs on this machine"
    } else {
        "Local route is not ready"
    }
}

fn flag(path: &str, body: &str, key: &str) -> Option<bool> {
    query_flag(path, key).or_else(|| body_flag(body, key))
}

fn query_flag(path: &str, key: &str) -> Option<bool> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key).then(|| value == "true")
    })
}

fn body_flag(body: &str, key: &str) -> Option<bool> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(key)?
        .as_bool()
}
