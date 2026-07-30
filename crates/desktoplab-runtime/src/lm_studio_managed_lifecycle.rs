use crate::{
    LmStudioManagedPlan, LmStudioModelProbe, ProcessCommand, ProcessRunner,
    RuntimeInstallExecutionResult,
    lm_studio_managed_execution::{managed, managed_daemon_pid},
    lm_studio_managed_marker::{clear_marker, read_marker},
    lm_studio_managed_status::{LmStudioManagedState, inspect_managed_lm_studio},
};

#[must_use]
pub fn stop_managed_lm_studio(
    runner: &impl ProcessRunner,
    plan: &LmStudioManagedPlan,
) -> RuntimeInstallExecutionResult {
    let Ok(Some(marker)) = read_marker(plan.marker_path()) else {
        return RuntimeInstallExecutionResult::blocked(
            "lm-studio ownership marker unavailable",
            "DesktopLab will not stop an LM Studio process it does not own.",
        );
    };
    let status = runner.run(managed(daemon_status(plan), plan));
    if managed_daemon_pid(&status) != Some(marker.pid) {
        return RuntimeInstallExecutionResult::blocked(
            status.evidence().evidence(),
            "The running LM Studio process does not match DesktopLab's ownership marker.",
        );
    }
    let server = runner.run(managed(
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("server")
            .arg("stop"),
        plan,
    ));
    let daemon = runner.run(managed(
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("daemon")
            .arg("down"),
        plan,
    ));
    let evidence = format!(
        "{}; {}",
        server.evidence().evidence(),
        daemon.evidence().evidence()
    );
    if !server.succeeded() || !daemon.succeeded() || clear_marker(plan.marker_path()).is_err() {
        return RuntimeInstallExecutionResult::failed(
            "managed_stop_failed",
            evidence,
            "DesktopLab could not stop its managed LM Studio process cleanly.",
        );
    }
    RuntimeInstallExecutionResult::verified_existing(evidence)
}

pub(crate) fn recover_managed_lm_studio(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
    plan: &LmStudioManagedPlan,
) -> Option<RuntimeInstallExecutionResult> {
    let marker = read_marker(plan.marker_path()).ok().flatten()?;
    let status = inspect_managed_lm_studio(runner, model_probe, plan)?;
    if status.state() != LmStudioManagedState::Ready {
        clear_marker(plan.marker_path()).ok()?;
        return None;
    }
    Some(RuntimeInstallExecutionResult::verified_existing(format!(
        "managed LM Studio recovered; pid={}; endpoint={}",
        marker.pid, marker.endpoint
    )))
}

fn daemon_status(plan: &LmStudioManagedPlan) -> ProcessCommand {
    ProcessCommand::new(plan.lms_path().to_string_lossy())
        .arg("daemon")
        .arg("status")
        .arg("--json")
}
