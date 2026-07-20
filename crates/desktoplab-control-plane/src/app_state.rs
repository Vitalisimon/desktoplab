use desktoplab_domain::ApprovalMode;
use serde_json::{Value, json};

use crate::{BackendReadinessState, setup_pipeline::SetupPipeline, setup_state::SetupState};

#[must_use]
pub fn app_state_json(
    setup: &SetupState,
    setup_pipeline: &SetupPipeline,
    readiness_evidence: &BackendReadinessState,
    current_workspace: Option<Value>,
    workspaces: Vec<Value>,
    active_approval_count: usize,
    active_session_count: usize,
    approval_modes: Value,
    session_approval_mode: ApprovalMode,
) -> Value {
    let readiness = if setup.is_ready() && readiness_evidence.is_ready() {
        "ready"
    } else {
        "blocked"
    };
    let setup_json = setup_projection(setup, readiness_evidence);
    let has_workspace = current_workspace.is_some();
    json!({
        "readiness":{"state":readiness,"evidence":readiness_evidence.to_json()},
        "setup":setup_json,
        "setupPipeline":setup_pipeline.to_json(),
        "currentWorkspace":current_workspace,
        "workspaces":workspaces,
        "approvalModes":approval_modes,
        "routeInput":{
            "readiness":readiness,
            "setupState":setup_projection(setup, readiness_evidence)["state"].clone(),
            "hasWorkspace":has_workspace,
            "activeApprovalCount":active_approval_count,
            "activeSessionCount":active_session_count,
            "approvalMode":session_approval_mode.as_str()
        }
    })
}

fn setup_projection(setup: &SetupState, readiness: &BackendReadinessState) -> Value {
    let mut setup_json = setup.to_json();
    if setup.is_ready() && !readiness.is_ready() {
        setup_json["state"] = json!("blocked");
        setup_json["blockedReason"] = json!("backend_readiness_not_verified");
    }
    setup_json
}
