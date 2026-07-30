use crate::{
    LmStudioModelProbe, MlxLmManagedPlan, ProcessCommand, ProcessRunner,
    mlx_lm_managed_execution::managed, mlx_lm_managed_marker::read_marker,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MlxLmManagedState {
    Ready,
    StaleOwnership,
    ModelUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmManagedStatus {
    state: MlxLmManagedState,
    endpoint: String,
    models: Vec<String>,
    pid: u32,
    model_revision: String,
}

impl MlxLmManagedStatus {
    #[must_use]
    pub fn state(&self) -> MlxLmManagedState {
        self.state
    }

    #[must_use]
    pub fn ready(&self) -> bool {
        self.state == MlxLmManagedState::Ready
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
    pub fn pid(&self) -> u32 {
        self.pid
    }

    #[must_use]
    pub fn model_revision(&self) -> &str {
        &self.model_revision
    }

    #[must_use]
    pub fn ownership(&self) -> &'static str {
        "desktoplab_managed"
    }
}

#[must_use]
pub fn inspect_managed_mlx_lm(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
    plan: &MlxLmManagedPlan,
) -> Option<MlxLmManagedStatus> {
    let marker = read_marker(&plan.marker_path()).ok().flatten()?;
    let process = runner.run(managed(
        ProcessCommand::new("ps")
            .arg("-p")
            .arg(marker.pid.to_string())
            .arg("-o")
            .arg("command="),
        plan,
    ));
    let process_matches = process.succeeded()
        && process
            .stdout()
            .contains(&plan.server_path().to_string_lossy().to_string())
        && process
            .stdout()
            .contains(&format!("--port {}", plan.port()));
    if !marker.matches_plan(plan) || !process_matches {
        return Some(MlxLmManagedStatus {
            state: MlxLmManagedState::StaleOwnership,
            endpoint: marker.endpoint,
            models: Vec::new(),
            pid: marker.pid,
            model_revision: marker.model_revision,
        });
    }
    let models = model_probe.models(&plan.endpoint()).unwrap_or_default();
    let state = if models.is_empty() {
        MlxLmManagedState::ModelUnavailable
    } else {
        MlxLmManagedState::Ready
    };
    Some(MlxLmManagedStatus {
        state,
        endpoint: marker.endpoint,
        models,
        pid: marker.pid,
        model_revision: marker.model_revision,
    })
}
