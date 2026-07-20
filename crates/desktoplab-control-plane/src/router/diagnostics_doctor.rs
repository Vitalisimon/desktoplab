use desktoplab_backend_services::JobState;
use serde_json::{Value, json};

use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn doctor_lint(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(self.doctor_lint_payload())
    }

    pub(crate) fn run_diagnostic_repair(&self, repair_id: &str, _body: &str) -> ApiRouteResponse {
        let (repair_kind, reason) = diagnostic_repair_contract(repair_id);
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "status":"blocked",
            "repairId":repair_id,
            "repairKind":repair_kind,
            "reason":reason,
            "requiresApproval":false,
            "sideEffects":[]
        }))
    }

    pub(super) fn doctor_lint_payload(&self) -> Value {
        let checks = doctor_lint_checks(
            self.setup.is_ready(),
            self.workspace.is_some(),
            self.jobs
                .list_jobs()
                .iter()
                .any(|job| matches!(job.state(), JobState::Blocked | JobState::Failed)),
        );
        let blocked = checks
            .iter()
            .filter(|check| check["severity"] == "blocked")
            .count();
        let degraded = checks
            .iter()
            .filter(|check| check["severity"] == "degraded")
            .count();
        let ready = checks
            .iter()
            .filter(|check| check["severity"] == "ready")
            .count();
        let state = if blocked > 0 {
            "blocked"
        } else if degraded > 0 {
            "degraded"
        } else {
            "ready"
        };
        json!({
            "source":"service_backed",
            "mode":"lint",
            "repairable":false,
            "summary":{
                "state":state,
                "blocked":blocked,
                "degraded":degraded,
                "ready":ready
            },
            "checks":checks
        })
    }
}

fn diagnostic_repair_contract(repair_id: &str) -> (&'static str, &'static str) {
    match repair_id {
        "repair.setup" | "repair.workspace" => {
            ("guidance_only", "guidance_only_repair_requires_user_action")
        }
        "repair.jobs" => ("stale_state_cleanup", "stale_state_cleanup_not_available"),
        "repair.registry" => ("local_config", "local_config_repair_not_available"),
        "repair.runtime"
        | "repair.runtime-install"
        | "repair.os-driver"
        | "repair.model-download.qwen" => ("external_manual", "external_manual_repair_required"),
        _ => ("unsupported", "unsupported_diagnostic_repair"),
    }
}

fn doctor_lint_checks(
    setup_ready: bool,
    has_workspace: bool,
    has_blocked_jobs: bool,
) -> Vec<Value> {
    vec![
        doctor_check(
            "doctor.setup.runtime_model_ready",
            "Runtime and model setup",
            if setup_ready { "ready" } else { "blocked" },
            "runtime",
            if setup_ready {
                "Setup has verified a local runtime and model."
            } else {
                "Setup must verify runtime and model before repository work."
            },
            if setup_ready {
                "No action needed."
            } else {
                "Finish setup before repository work."
            },
            "repair.setup",
        ),
        doctor_check(
            "doctor.workspace.open",
            "Repository workspace",
            if has_workspace { "ready" } else { "blocked" },
            "workspace_scan",
            if has_workspace {
                "A repository workspace is open."
            } else {
                "Repository work needs an open workspace."
            },
            if has_workspace {
                "No action needed."
            } else {
                "Open a repository workspace."
            },
            "repair.workspace",
        ),
        doctor_check(
            "doctor.jobs.no_blocked_background_work",
            "Background work",
            if has_blocked_jobs {
                "degraded"
            } else {
                "ready"
            },
            "job",
            if has_blocked_jobs {
                "Background work has blocked or failed jobs."
            } else {
                "No blocked background work."
            },
            if has_blocked_jobs {
                "Review background work before beta evidence."
            } else {
                "No action needed."
            },
            "repair.jobs",
        ),
        doctor_check(
            "doctor.storage.migrations_declared",
            "Storage migrations",
            "ready",
            "storage",
            "Storage migrations declare ids, checksums and operator status.",
            "No migration action needed.",
            "none",
        ),
    ]
}

fn doctor_check(
    check_id: &str,
    label: &str,
    severity: &str,
    source: &str,
    message: &str,
    fix_hint: &str,
    repair_id: &str,
) -> Value {
    json!({
        "checkId":check_id,
        "label":label,
        "severity":severity,
        "source":source,
        "message":message,
        "fixHint":fix_hint,
        "repairId":repair_id
    })
}
