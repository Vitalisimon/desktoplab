use serde_json::{Value, json};

use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn security_audit(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(security_audit_payload(
            self.setup.is_ready(),
            self.workspace.is_some(),
            self.session_approval_mode.as_str(),
        ))
    }
}

fn security_audit_payload(setup_ready: bool, has_workspace: bool, approval_mode: &str) -> Value {
    let findings = vec![
        finding(
            "security.local_only.posture",
            "Local-only posture",
            "ready",
            "local_control_plane",
            "Local control-plane routes are modeled as local-first and redacted.",
            "No action needed.",
            "none",
            false,
        ),
        finding(
            "security.provider_egress.approval_gated",
            "Provider egress",
            "ready",
            "provider_policy",
            "External provider egress requires policy approval before use.",
            "Keep provider fallback approvals explicit.",
            "none",
            false,
        ),
        finding(
            "security.approval_mode.current",
            "Approval mode",
            if approval_mode == "full_access" {
                "degraded"
            } else {
                "ready"
            },
            "approval_policy",
            &format!("Current session approval mode is {approval_mode}."),
            "Use conservative approval for beta evidence when possible.",
            "none",
            false,
        ),
        finding(
            "security.workspace.protected_paths",
            "Protected paths",
            if has_workspace { "ready" } else { "degraded" },
            "workspace_policy",
            if has_workspace {
                "Protected local-only paths are blocked before execution."
            } else {
                "Protected path policy is present; no workspace is open."
            },
            "Open a workspace to verify repository-scoped checks.",
            "repair.workspace",
            false,
        ),
        finding(
            "security.plugins.provenance",
            "Plugin provenance",
            "blocked",
            "plugin_policy",
            "Executable plugin runtime is not certified; descriptor surfaces stay non-executable.",
            "Keep plugin execution disabled until provenance and sandbox gates exist.",
            "none",
            true,
        ),
        finding(
            "security.backends.trust_level",
            "Backend trust level",
            if setup_ready { "ready" } else { "degraded" },
            "execution_backend",
            if setup_ready {
                "Selected backend has a declared support contract."
            } else {
                "Backend support contract exists, but setup is not complete."
            },
            "Complete setup before private beta evidence.",
            "repair.setup",
            false,
        ),
        finding(
            "security.redaction.export_ready",
            "Redaction and export",
            "ready",
            "redaction",
            "Diagnostics export is bounded and marked redacted for review before sharing.",
            "Review bundles before sharing with maintainers.",
            "none",
            false,
        ),
    ];
    let blocked = count(&findings, "blocked");
    let degraded = count(&findings, "degraded");
    let state = if blocked > 0 {
        "blocked"
    } else if degraded > 0 {
        "degraded"
    } else {
        "ready"
    };
    json!({
        "source":"service_backed",
        "kind":"security_audit",
        "redacted":true,
        "exportSafe":true,
        "summary":{
            "state":state,
            "blocked":blocked,
            "degraded":degraded,
            "ready":count(&findings, "ready")
        },
        "findings":findings,
        "remediationPolicy":"safe_remediation_routes_through_doctor_repair_contract"
    })
}

fn finding(
    check_id: &str,
    label: &str,
    severity: &str,
    source: &str,
    message: &str,
    fix_hint: &str,
    repair_id: &str,
    suppressed: bool,
) -> Value {
    json!({
        "checkId":check_id,
        "label":label,
        "severity":severity,
        "source":source,
        "message":message,
        "fixHint":fix_hint,
        "repairId":repair_id,
        "suppressed":suppressed
    })
}

fn count(findings: &[Value], severity: &str) -> usize {
    findings
        .iter()
        .filter(|finding| finding["severity"] == severity && finding["suppressed"] != true)
        .count()
}
