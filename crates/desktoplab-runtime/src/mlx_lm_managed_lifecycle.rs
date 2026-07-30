use crate::{
    LmStudioModelProbe, MlxLmManagedPlan, ProcessCommand, ProcessRunner,
    RuntimeInstallExecutionResult,
    mlx_lm_managed_execution::managed,
    mlx_lm_managed_marker::{clear_marker, read_marker},
    mlx_lm_managed_status::{MlxLmManagedState, inspect_managed_mlx_lm},
};

#[must_use]
pub fn stop_managed_mlx_lm(
    runner: &impl ProcessRunner,
    plan: &MlxLmManagedPlan,
) -> RuntimeInstallExecutionResult {
    let Ok(Some(marker)) = read_marker(&plan.marker_path()) else {
        return RuntimeInstallExecutionResult::blocked(
            "mlx-lm ownership marker unavailable",
            "DesktopLab will not stop an MLX-LM process it does not own.",
        );
    };
    let process = runner.run(managed(
        ProcessCommand::new("ps")
            .arg("-p")
            .arg(marker.pid.to_string())
            .arg("-o")
            .arg("command="),
        plan,
    ));
    if !marker.matches_plan(plan)
        || !process.succeeded()
        || !process
            .stdout()
            .contains(&plan.server_path().to_string_lossy().to_string())
        || !process
            .stdout()
            .contains(&format!("--port {}", plan.port()))
    {
        return RuntimeInstallExecutionResult::blocked(
            process.evidence().evidence(),
            "The running process does not match DesktopLab's MLX-LM ownership marker.",
        );
    }
    let stop = runner.run(
        ProcessCommand::new("kill")
            .arg("-TERM")
            .arg(marker.pid.to_string()),
    );
    if !stop.succeeded() || clear_marker(&plan.marker_path()).is_err() {
        return RuntimeInstallExecutionResult::failed(
            "managed_stop_failed",
            stop.evidence().evidence(),
            "DesktopLab could not stop its managed MLX-LM process cleanly.",
        );
    }
    RuntimeInstallExecutionResult::verified_existing(stop.evidence().evidence())
}

pub(crate) fn recover_managed_mlx_lm(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
    plan: &MlxLmManagedPlan,
) -> Option<RuntimeInstallExecutionResult> {
    read_marker(&plan.marker_path()).ok().flatten()?;
    let status = inspect_managed_mlx_lm(runner, model_probe, plan)?;
    if status.state() != MlxLmManagedState::Ready {
        clear_marker(&plan.marker_path()).ok()?;
        return None;
    }
    Some(RuntimeInstallExecutionResult::verified_existing(format!(
        "managed MLX-LM recovered; pid={}; endpoint={}; revision={}",
        status.pid(),
        status.endpoint(),
        status.model_revision()
    )))
}
