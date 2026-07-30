use crate::{
    LmStudioManagedPlan, LmStudioModelProbe, ProcessCommand, ProcessRunner,
    lm_studio_managed_execution::{managed, managed_daemon_pid},
    lm_studio_managed_marker::read_marker,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LmStudioManagedState {
    Ready,
    StaleOwnership,
    ModelUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioManagedStatus {
    state: LmStudioManagedState,
    endpoint: String,
    models: Vec<String>,
    pid: u64,
    version: String,
}

impl LmStudioManagedStatus {
    #[must_use]
    pub fn state(&self) -> LmStudioManagedState {
        self.state
    }

    #[must_use]
    pub fn ready(&self) -> bool {
        self.state == LmStudioManagedState::Ready
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn models(&self) -> &[String] {
        &self.models
    }

    #[must_use]
    pub fn pid(&self) -> u64 {
        self.pid
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn ownership(&self) -> &'static str {
        "desktoplab_managed"
    }
}

#[must_use]
pub fn inspect_managed_lm_studio(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
    plan: &LmStudioManagedPlan,
) -> Option<LmStudioManagedStatus> {
    let marker = read_marker(plan.marker_path()).ok().flatten()?;
    let status = runner.run(managed(daemon_status(plan), plan));
    if !marker.matches_plan(plan) || managed_daemon_pid(&status) != Some(marker.pid) {
        return Some(LmStudioManagedStatus {
            state: LmStudioManagedState::StaleOwnership,
            endpoint: marker.endpoint,
            models: Vec::new(),
            pid: marker.pid,
            version: marker.artifact_version,
        });
    }
    let models = model_probe.models(&plan.endpoint()).unwrap_or_default();
    let state = if models.iter().any(|model| model == plan.api_model_id()) {
        LmStudioManagedState::Ready
    } else {
        LmStudioManagedState::ModelUnavailable
    };
    Some(LmStudioManagedStatus {
        state,
        endpoint: marker.endpoint,
        models,
        pid: marker.pid,
        version: marker.artifact_version,
    })
}

fn daemon_status(plan: &LmStudioManagedPlan) -> ProcessCommand {
    ProcessCommand::new(plan.lms_path().to_string_lossy())
        .arg("daemon")
        .arg("status")
        .arg("--json")
}
