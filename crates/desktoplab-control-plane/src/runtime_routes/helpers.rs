use desktoplab_runtime::{
    InstallPlan, InstallerSource, MlxLmRuntime, RuntimeExecutionState, RuntimeId,
    RuntimeInstallError, RuntimeInstallExecutionStrategy, RuntimeState, SystemProcessRunner,
};

mod body_fields;
mod host_target;

pub(super) use body_fields::{bool_body_field, number_body_field, segment, string_body_field};
pub(super) use host_target::{host_supports_mlx_lm, host_target};

pub(super) fn plan_for_runtime(runtime_id: &str) -> InstallPlan {
    if runtime_id == "runtime.mlx-lm" {
        return MlxLmRuntime::new()
            .try_install_plan(host_target())
            .unwrap_or_else(|_| {
                InstallPlan::new(RuntimeId::new(runtime_id), "MLX-LM Server")
                    .with_step("blocked: Apple Silicon Mac required")
                    .with_installer_source(InstallerSource::signed_url(
                        "https://pypi.org/project/mlx-lm/",
                        "pypi-release-metadata",
                    ))
                    .with_execution_strategy(RuntimeInstallExecutionStrategy::PythonEnvironment)
                    .with_verification_step("Apple Silicon Mac required")
            });
    }
    let name = match runtime_id {
        "runtime.lm-studio" => "LM Studio",
        _ => "Ollama",
    };
    InstallPlan::new(RuntimeId::new(runtime_id), name)
        .with_step("download signed installer")
        .with_disk_requirement_gb(2)
        .with_network_required(true)
        .with_installer_source(InstallerSource::signed_url(
            format!("https://registry.desktoplab.local/{runtime_id}.tar.gz"),
            "sha256:desktoplab-runtime",
        ))
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_verification_step("verify runtime health")
}

pub(super) fn runtime_state(state: RuntimeState) -> &'static str {
    match state {
        RuntimeState::NotInstalled => "not_installed",
        RuntimeState::Installed => "installed",
        RuntimeState::Running => "running",
        RuntimeState::Ready => "ready",
        RuntimeState::VerificationFailed => "verification_failed",
        RuntimeState::Degraded => "degraded",
        RuntimeState::Starting => "starting",
        RuntimeState::Stopped => "stopped",
        RuntimeState::Unknown => "unknown",
    }
}

pub(super) fn host_runtime_state(runtime_id: &str, fallback: RuntimeState) -> &'static str {
    if runtime_id == "runtime.mlx-lm" {
        let output = <SystemProcessRunner as desktoplab_runtime::ProcessRunner>::run(
            &SystemProcessRunner,
            desktoplab_runtime::ProcessCommand::new("python3")
                .arg("-c")
                .arg("import mlx_lm; print('mlx-lm import ok')"),
        );
        return if output.succeeded() {
            "installed"
        } else {
            runtime_state(fallback)
        };
    }
    if runtime_id != "runtime.ollama" {
        return "degraded";
    }
    let output = <SystemProcessRunner as desktoplab_runtime::ProcessRunner>::run(
        &SystemProcessRunner,
        desktoplab_runtime::ProcessCommand::new("ollama").arg("--version"),
    );
    if !output.succeeded() {
        return runtime_state(fallback);
    }
    let health = <SystemProcessRunner as desktoplab_runtime::ProcessRunner>::run(
        &SystemProcessRunner,
        desktoplab_runtime::ProcessCommand::new("curl")
            .arg("--fail")
            .arg("http://127.0.0.1:11434/api/tags"),
    );
    if health.succeeded() {
        "ready"
    } else {
        "degraded"
    }
}

pub(super) fn execution_state(state: RuntimeExecutionState) -> &'static str {
    match state {
        RuntimeExecutionState::Completed => "completed",
        RuntimeExecutionState::Blocked => "blocked",
        RuntimeExecutionState::ExternalGuided => "external_guided",
        RuntimeExecutionState::Failed => "failed",
    }
}

pub(super) fn runtime_execution_retry_class(state: RuntimeExecutionState) -> &'static str {
    match state {
        RuntimeExecutionState::Completed => "none",
        RuntimeExecutionState::Blocked => "user_action",
        RuntimeExecutionState::ExternalGuided => "user_action",
        RuntimeExecutionState::Failed => "retryable",
    }
}

pub(super) fn runtime_rank(runtime_id: &str) -> u8 {
    match runtime_id {
        "runtime.ollama" => 0,
        "runtime.mlx-lm" => 1,
        "runtime.lm-studio" => 2,
        _ => 2,
    }
}

pub(super) fn runtime_install_error_reason(error: &RuntimeInstallError) -> &'static str {
    match error {
        RuntimeInstallError::SetupPlanNotAccepted => "setup plan not accepted",
        RuntimeInstallError::MissingVerificationMetadata => "missing verification metadata",
        RuntimeInstallError::UnknownSetupChoice => "unknown setup choice",
        RuntimeInstallError::UnsafeInstallerSource => "unsafe installer source",
        RuntimeInstallError::ExternallyManagedRuntime => "runtime is externally managed",
        RuntimeInstallError::InsufficientDisk { .. } => "insufficient disk",
        RuntimeInstallError::NetworkUnavailable => "network unavailable",
    }
}

pub(super) fn runtime_install_retry_class(error: &RuntimeInstallError) -> &'static str {
    match error {
        RuntimeInstallError::NetworkUnavailable => "offline",
        RuntimeInstallError::InsufficientDisk { .. } => "user_action",
        RuntimeInstallError::UnsafeInstallerSource => "non_retryable",
        _ => "non_retryable",
    }
}
