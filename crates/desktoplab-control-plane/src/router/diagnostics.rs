use desktoplab_backend_services::{AuditQuery, JobState};
use serde_json::{Value, json};

use super::helpers::job_state_value;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn local_audit(&self) -> ApiRouteResponse {
        let snapshot = self.audit.transparency_snapshot(AuditQuery::all(), 50);
        let records = snapshot
            .records
            .iter()
            .map(|record| {
                json!({
                    "sequence":record.sequence,
                    "action":record.action,
                    "outcome":record.outcome,
                    "redactedDetails":record.redacted_details
                })
            })
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({
            "scope":snapshot.scope,
            "records":records,
            "redactedExport":snapshot.redacted_export
        }))
    }

    pub(crate) fn diagnostics_snapshot(&self) -> ApiRouteResponse {
        let setup_ready = self.setup.is_ready();
        let has_workspace = self.workspace.is_some();
        let jobs = self.jobs.list_jobs();
        let active_jobs = jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.state(),
                    JobState::Queued | JobState::Running | JobState::AwaitingApproval
                )
            })
            .count();
        let blocked_jobs = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::Blocked | JobState::Failed))
            .count();
        let job_summary = if jobs.is_empty() {
            "none".to_string()
        } else {
            jobs.iter()
                .map(|job| format!("{}:{}", job.kind(), job_state_value(job.state())))
                .collect::<Vec<_>>()
                .join(",")
        };
        let state = if !setup_ready && jobs.is_empty() {
            "blocked"
        } else if !setup_ready || active_jobs > 0 || blocked_jobs > 0 {
            "degraded"
        } else {
            "ready"
        };
        let journal_fault = self.state_journal_failure();
        let state = if journal_fault.is_some() {
            "degraded"
        } else {
            state
        };
        ApiRouteResponse::ok(json!({
            "state":state,
            "services":[
                service("runtime", "Runtime", runtime_state(setup_ready), runtime_message(setup_ready)),
                service("model", "Model", model_state(setup_ready), model_message(setup_ready)),
                service("workspace_scan", "Workspace", if has_workspace {"ready"} else {"blocked"}, if has_workspace {"Repository is open."} else {"Open a repository after setup."}),
                service("job", "Background work", job_service_state(active_jobs, blocked_jobs), &job_message(active_jobs, blocked_jobs)),
                service(
                    "state_journal",
                    "State journal",
                    if journal_fault.is_some() {"failed"} else {"ready"},
                    journal_fault.as_deref().unwrap_or("Persistent state is healthy.")
                )
            ],
            "repairActions":repair_actions(setup_ready, has_workspace, blocked_jobs),
            "bundlePreview":{
                "summary":format!(
                    "setup={} workspace={} jobs={} token=[REDACTED]",
                    if setup_ready {"ready"} else {"not_ready"},
                    if has_workspace {"open"} else {"none"},
                    job_summary
                ),
                "setup":{
                    "runtimeId":self.setup.to_json()["runtimeId"].clone(),
                    "modelId":self.setup.to_json()["modelId"].clone(),
                    "pipelineState":self.setup_pipeline.to_json()["state"].clone()
                },
                "hardware":[
                    {"label":"OS","value":std::env::consts::OS,"confidence":"confirmed"},
                    {"label":"Architecture","value":std::env::consts::ARCH,"confidence":"confirmed"}
                ],
                "jobs":jobs.iter().map(|job| json!({
                    "kind":job.kind(),
                    "state":job_state_value(job.state())
                })).collect::<Vec<_>>(),
                "redactedErrors":jobs.iter()
                    .filter(|job| matches!(job.state(), JobState::Blocked | JobState::Failed))
                    .map(|job| {
                        json!({
                            "kind":job.kind(),
                            "message":"Background work needs attention.",
                            "redacted":true
                        })
                    })
                    .collect::<Vec<_>>(),
                "sizeBytes":512,
                "maxBytes":64000,
                "redacted":true
            },
            "updateStatus":{
                "channel":"dev",
                "currentVersion":"0.1.0",
                "state":"disabled",
                "message":"In-app updates are disabled for this build. Install future builds manually until a signed hosted channel is enabled.",
                "canInstall":false
            },
            "doctorLint":self.doctor_lint_payload(),
            "migrationStatus":self.migration_status_payload(),
            "stability":self.stability_snapshot_payload(),
            "localAudit":self.local_audit_payload()
        }))
    }

    fn local_audit_payload(&self) -> Value {
        let snapshot = self.audit.transparency_snapshot(AuditQuery::all(), 50);
        json!({
            "scope":snapshot.scope,
            "records":snapshot.records.iter().map(|record| json!({
                "sequence":record.sequence,
                "action":record.action,
                "outcome":record.outcome,
                "redactedDetails":record.redacted_details
            })).collect::<Vec<_>>(),
            "redactedExport":snapshot.redacted_export
        })
    }
}

fn service(family: &str, label: &str, state: &str, message: &str) -> Value {
    json!({"family":family,"label":label,"state":state,"message":message})
}

fn runtime_state(setup_ready: bool) -> &'static str {
    if setup_ready { "ready" } else { "blocked" }
}

fn model_state(setup_ready: bool) -> &'static str {
    if setup_ready { "ready" } else { "blocked" }
}

fn runtime_message(setup_ready: bool) -> &'static str {
    if setup_ready {
        "Runtime readiness is verified."
    } else {
        "Setup has not verified a local runtime yet."
    }
}

fn model_message(setup_ready: bool) -> &'static str {
    if setup_ready {
        "Model readiness is verified."
    } else {
        "Setup has not verified a model yet."
    }
}

fn job_service_state(active_jobs: usize, blocked_jobs: usize) -> &'static str {
    if blocked_jobs > 0 {
        "degraded"
    } else if active_jobs > 0 {
        "degraded"
    } else {
        "ready"
    }
}

fn job_message(active_jobs: usize, blocked_jobs: usize) -> String {
    if blocked_jobs > 0 {
        let noun = if blocked_jobs == 1 {
            "job needs"
        } else {
            "jobs need"
        };
        format!("{blocked_jobs} background {noun} review.")
    } else if active_jobs > 0 {
        let noun = if active_jobs == 1 {
            "job is"
        } else {
            "jobs are"
        };
        format!("{active_jobs} background {noun} running.")
    } else {
        "No active background work.".to_string()
    }
}

fn repair_actions(setup_ready: bool, has_workspace: bool, blocked_jobs: usize) -> Vec<Value> {
    let mut actions = Vec::new();
    if !setup_ready {
        actions.push(json!({
            "repairId":"repair.setup",
            "family":"runtime",
            "label":"Finish setup",
            "reason":"Runtime and model must be verified before repository work.",
            "mode":"guidance_only",
            "repairKind":"guidance_only"
        }));
    }
    if !has_workspace {
        actions.push(json!({
            "repairId":"repair.workspace",
            "family":"workspace_scan",
            "label":"Open repository",
            "reason":"Workspace diagnostics start after a repository is open.",
            "mode":"guidance_only",
            "repairKind":"guidance_only"
        }));
    }
    if blocked_jobs > 0 {
        actions.push(json!({
            "repairId":"repair.jobs",
            "family":"job",
            "label":"Review background work",
            "reason":"A background job is blocked or failed.",
            "mode":"guidance_only",
            "repairKind":"stale_state_cleanup"
        }));
    }
    actions
}
