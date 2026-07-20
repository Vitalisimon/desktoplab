use desktoplab_backend_services::{AuditQuery, JobState};
use serde_json::json;

use super::helpers::job_state_value;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn diagnostics_export(&self) -> ApiRouteResponse {
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
        let setup_ready = self.setup.is_ready();
        let has_workspace = self.workspace.is_some();
        let audit = self.audit.transparency_snapshot(AuditQuery::all(), 25);
        let state = if !setup_ready && jobs.is_empty() {
            "blocked"
        } else if !setup_ready || active_jobs > 0 || blocked_jobs > 0 {
            "degraded"
        } else {
            "ready"
        };
        ApiRouteResponse::ok(json!({
            "manifest":{
                "kind":"desktoplab.diagnostics.export",
                "schemaVersion":1,
                "redactionProfile":"private_beta_local"
            },
            "summary":{
                "state":state,
                "redacted":true,
                "sizeBytes":1024,
                "maxBytes":64000
            },
            "serviceStates":[
                {"family":"runtime","state":if setup_ready {"ready"} else {"blocked"}},
                {"family":"model","state":if setup_ready {"ready"} else {"blocked"}},
                {"family":"workspace_scan","state":if has_workspace {"ready"} else {"blocked"}},
                {"family":"job","state":if blocked_jobs > 0 || active_jobs > 0 {"degraded"} else {"ready"}}
            ],
            "routeFacts":{
                "selectedRouteId":self.selected_route_id,
                "egress":"local_or_approval_gated",
                "backendOwner":"desktoplab"
            },
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
            "redactedAuditRecords":audit.records.iter().map(|record| json!({
                "sequence":record.sequence,
                "action":record.action,
                "outcome":record.outcome,
                "redactedDetails":record.redacted_details
            })).collect::<Vec<_>>(),
            "redaction":{
                "promptsIncluded":false,
                "rawToolOutputIncluded":false,
                "secretsIncluded":false,
                "privatePathsIncluded":false
            },
            "reviewBeforeSharing":true
        }))
    }
}
