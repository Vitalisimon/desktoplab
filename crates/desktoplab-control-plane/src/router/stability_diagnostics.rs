use desktoplab_backend_services::JobState;
use serde_json::{Value, json};

use crate::lifecycle::StabilityBudget;

use super::LocalApiRouter;
use super::helpers::job_state_value;

impl LocalApiRouter {
    pub(crate) fn stability_snapshot_payload(&self) -> Value {
        let jobs = self.jobs.list_jobs();
        let queued = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::Queued))
            .count();
        let running = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::Running))
            .count();
        let awaiting_approval = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::AwaitingApproval))
            .count();
        let blocked = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::Blocked))
            .count();
        let failed = jobs
            .iter()
            .filter(|job| matches!(job.state(), JobState::Failed))
            .count();
        let active = queued + running + awaiting_approval;
        let attention = blocked + failed;
        let budget = StabilityBudget::default();
        json!({
            "kind":"desktoplab.stability.snapshot",
            "schemaVersion":1,
            "redacted":true,
            "payloadFree":true,
            "startupPhase":startup_phase(self.setup.is_ready(), self.workspace.is_some()),
            "uptimeMs":self.stability.uptime_ms(),
            "localApiHealth":{
                "state":"responding",
                "scope":"loopback_router",
                "payloadFree":true
            },
            "routeDecisionRecency":{
                "state":"current",
                "selectedRouteId":self.selected_route_id,
                "lastChangedAgoMs":self.stability.route_decision_age_ms()
            },
            "queueBackpressure":{
                "state":backpressure_state(active, attention),
                "queued":queued,
                "running":running,
                "awaitingApproval":awaiting_approval,
                "blocked":blocked,
                "failed":failed,
                "active":active,
                "payloadFree":true
            },
            "budgets":{
                "memory":{"budgetMb":budget.memory_budget_mb(),"sampleState":"not_sampled"},
                "disk":{"minimumFreeMb":budget.disk_budget_mb(),"sampleState":"not_sampled"}
            },
            "degradedReasons":degraded_reasons(self.setup.is_ready(), self.workspace.is_some(), active, attention),
            "jobStates":jobs.iter().map(|job| json!({
                "kind":job.kind(),
                "state":job_state_value(job.state())
            })).collect::<Vec<_>>()
        })
    }
}

fn startup_phase(setup_ready: bool, has_workspace: bool) -> &'static str {
    if !setup_ready {
        "setup_pending"
    } else if !has_workspace {
        "workspace_pending"
    } else {
        "ready"
    }
}

fn backpressure_state(active: usize, attention: usize) -> &'static str {
    if attention > 0 {
        "attention_required"
    } else if active > 0 {
        "busy"
    } else {
        "idle"
    }
}

fn degraded_reasons(
    setup_ready: bool,
    has_workspace: bool,
    active: usize,
    attention: usize,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if !setup_ready {
        reasons.push("setup_not_ready");
    }
    if !has_workspace {
        reasons.push("workspace_not_open");
    }
    if active > 0 {
        reasons.push("background_work_active");
    }
    if attention > 0 {
        reasons.push("background_work_needs_attention");
    }
    reasons
}
