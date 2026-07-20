use desktoplab_backend_services::{
    BackendRouteCandidate, BackendRouteDecision, BackendRouteService, BackendRouteStatus,
    RouteApiPolicy, RouteApiRequest,
};
use serde_json::{Value, json};

use crate::execution_backend_capabilities::{backend_capability_profile, backend_support_contract};

#[must_use]
pub fn external_backends_response() -> Value {
    let decision = external_route_decision();
    let capability_profile = backend_capability_profile("backend.codex");
    json!({
        "source":"service_backed",
        "bridgeContract":external_agent_bridge_v2_contract(),
        "backends":[{
            "backendId":capability_profile.backend_id(),
            "displayName":"Codex bridge",
            "kind":"external",
            "status":"blocked",
            "capabilities":capability_profile.advertised(),
            "backendSupportContract":backend_support_contract(capability_profile.backend_id()),
            "routes":[{
                "routeId":"route.codex",
                "status":status_value(decision.status()),
                "reason":external_route_reason(&decision),
                "blockedReasons":decision.blocked_reasons()
            }]
        }]
    })
}

#[must_use]
pub fn external_agent_bridge_v2_contract() -> Value {
    json!({
        "source":"service_backed",
        "contractId":"external-agent-bridge-v2",
        "schemaVersion":2,
        "sessionOwner":"desktoplab",
        "executionOwner":"external_backend",
        "status":"contract_ready",
        "routePath":"/v1/external-backends/bridge-contract/v2",
        "normalizedEventKinds":[
            "agent.plan",
            "agent.tool_request",
            "agent.diff",
            "agent.validation",
            "agent.summary",
            "agent.blocked"
        ],
        "eventEnvelope":{
            "requiredFields":[
                "sessionId",
                "workspaceId",
                "backendId",
                "sequence",
                "kind",
                "payload",
                "evidence"
            ],
            "ordering":"monotonic_per_session",
            "redaction":"provider_output_redacted_before_persistence"
        },
        "approvalBoundary":{
            "repositoryContextEgress":"explicit_approval_required",
            "toolRequests":"desktoplab_policy_and_approval_required",
            "credentialMaterial":"vault_ref_only"
        },
        "capabilityMappingRequired":[
            "event_stream.normalized",
            "tool_request.delegated",
            "diff.proposed",
            "validation.reported",
            "session.resume"
        ],
        "unsupportedWithoutCertification":[
            "raw_token_ingress",
            "provider_owned_session",
            "automatic_repository_egress",
            "unapproved_tool_execution"
        ]
    })
}

pub(crate) fn codex_route_response(codex_bridge_ready: bool) -> Value {
    let capability_profile = backend_capability_profile("backend.codex");
    let status = if codex_bridge_ready {
        "selected"
    } else {
        "blocked"
    };
    json!({
        "source":"service_backed",
        "routeId":"route.external.codex",
        "status":status,
        "backendId":capability_profile.backend_id(),
        "backendDisplayName":"Codex bridge",
        "backendKind":"external",
        "modelDisplayName":"Codex bridge",
        "runtimeDisplayName":"Codex",
        "summary":if codex_bridge_ready { "Runs through the connected local Codex bridge" } else { "Codex bridge is not connected" },
        "reasons":if codex_bridge_ready { vec!["OpenAI Codex bridge is connected through a local responder."] } else { vec!["Connect the Codex bridge before routing work outside DesktopLab."] },
        "blockedReasons":if codex_bridge_ready { Vec::<String>::new() } else { vec!["codex_bridge_not_connected".to_string()] },
        "requiredCapabilities":capability_profile.required(),
        "backendCapabilities":capability_profile.advertised(),
        "backendSupportContract":backend_support_contract(capability_profile.backend_id()),
        "egressPolicy":"requires_approval",
        "repositoryContextEgress":"approval_required",
        "needsFallbackApproval":false
    })
}

fn external_route_decision() -> BackendRouteDecision {
    let capability_profile = backend_capability_profile("backend.codex");
    BackendRouteService::new(RouteApiPolicy::local_only()).plan(
        RouteApiRequest::new(capability_profile.required()).with_preferred_backend("backend.codex"),
        vec![
            BackendRouteCandidate::cloud(
                capability_profile.backend_id(),
                capability_profile.required(),
            )
            .mark_runtime_unavailable("credential missing"),
        ],
    )
}

fn status_value(status: BackendRouteStatus) -> &'static str {
    match status {
        BackendRouteStatus::Selected => "selected",
        BackendRouteStatus::Blocked => "blocked",
    }
}

fn external_route_reason(decision: &BackendRouteDecision) -> &'static str {
    if decision
        .blocked_reasons()
        .iter()
        .any(|reason| reason == "runtime_unavailable:credential missing")
    {
        "credential missing"
    } else {
        "route blocked"
    }
}
