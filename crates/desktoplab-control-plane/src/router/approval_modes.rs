use desktoplab_domain::ApprovalMode;
use serde_json::{Value, json};

use super::helpers::body_field;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn approval_modes(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(self.approval_modes_payload())
    }

    pub(crate) fn update_default_approval_mode(&mut self, body: &str) -> ApiRouteResponse {
        let Some(mode) = approval_mode_from_body(body) else {
            return invalid_approval_mode();
        };
        self.default_approval_mode = mode;
        self.session_approval_mode = mode;
        self.persist_default_approval_mode();
        ApiRouteResponse::ok(self.approval_modes_payload())
    }

    pub(crate) fn update_session_approval_mode(&mut self, body: &str) -> ApiRouteResponse {
        let Some(mode) = approval_mode_from_body(body) else {
            return invalid_approval_mode();
        };
        self.session_approval_mode = mode;
        ApiRouteResponse::ok(self.approval_modes_payload())
    }

    pub(crate) fn approval_modes_payload(&self) -> Value {
        json!({
            "modes":ApprovalMode::ALL.map(approval_mode_json),
            "defaultMode":self.default_approval_mode.as_str(),
            "sessionMode":self.session_approval_mode.as_str()
        })
    }
}

fn approval_mode_from_body(body: &str) -> Option<ApprovalMode> {
    body_field(body, "mode").and_then(|mode| ApprovalMode::from_stable_str(&mode))
}

fn approval_mode_json(mode: ApprovalMode) -> Value {
    json!({
        "mode":mode.as_str(),
        "label":mode.label(),
        "description":mode.description()
    })
}

fn invalid_approval_mode() -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"INVALID_APPROVAL_MODE",
        "message":"Unknown approval mode.",
        "allowedModes":ApprovalMode::ALL.map(ApprovalMode::as_str)
    }))
}
