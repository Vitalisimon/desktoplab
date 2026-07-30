use desktoplab_runtime::{
    InstallHostCapacity, OllamaRuntime, RuntimeExecutionState, RuntimeInstallError,
    RuntimeInstallExecutor, RuntimeInstallJob, RuntimeInstallPlanner, RuntimeInstallRequest,
    SystemProcessRunner,
};

mod helpers;
mod inventory;
mod lm_studio_managed;
pub(crate) mod mlx_lm_managed;
mod setup_choice;

pub use inventory::runtimes_response;
pub use setup_choice::{RuntimeSetupChoice, runtime_setup_choice};

use helpers::{
    bool_body_field, execution_state, host_target, number_body_field, plan_for_runtime,
    runtime_execution_retry_class, runtime_install_error_reason, runtime_install_retry_class,
    safe_cached_installer_reference, segment, string_body_field,
};
use serde_json::{Value, json};

pub fn plan_runtime_install(
    path: &str,
    body: &str,
) -> Result<(String, RuntimeInstallJob), RuntimeInstallError> {
    let runtime_id = segment(path, 2);
    if !crate::local_runtime_capability::LocalRuntimeCapability::for_runtime(&runtime_id)
        .allows_setup()
    {
        return Err(RuntimeInstallError::RuntimeNotAvailable);
    }
    if let Some(cached_installer_path) = string_body_field(body, "cachedInstallerPath")
        && !safe_cached_installer_reference(&cached_installer_path)
    {
        return Err(RuntimeInstallError::UnsafeInstallerSource);
    }
    let capacity = InstallHostCapacity::new(
        number_body_field(body, "diskAvailableGb").unwrap_or(64),
        bool_body_field(body, "networkAvailable").unwrap_or(true),
    );
    let planner = RuntimeInstallPlanner::new(capacity);
    let request = RuntimeInstallRequest::new(plan_for_runtime(&runtime_id))
        .with_setup_plan_accepted(bool_body_field(body, "setupAccepted").unwrap_or(true));
    planner.plan_job(request).map(|job| (runtime_id, job))
}

#[must_use]
pub fn execute_runtime_install(
    runtime_id: &str,
    setup_choice: RuntimeSetupChoice,
    body: &str,
) -> desktoplab_runtime::RuntimeInstallExecutionResult {
    if runtime_id == "runtime.lm-studio" {
        return lm_studio_managed::execute(body);
    }
    if runtime_id == "runtime.mlx-lm" {
        return mlx_lm_managed::execute(body);
    }
    let executor = RuntimeInstallExecutor::new(SystemProcessRunner);
    let plan = OllamaRuntime::new().platform_install_plan(host_target());
    if setup_choice == RuntimeSetupChoice::Replace {
        executor.execute_install(&plan)
    } else {
        executor.execute_existing_or_install(&plan)
    }
}

#[must_use]
pub fn verify_runtime(runtime_id: &str) -> desktoplab_runtime::RuntimeInstallExecutionResult {
    if runtime_id == "runtime.lm-studio" {
        return lm_studio_managed::verify_existing();
    }
    if runtime_id == "runtime.mlx-lm" {
        return mlx_lm_managed::verify();
    }
    RuntimeInstallExecutor::new(SystemProcessRunner).verify_existing(runtime_id)
}

#[must_use]
pub fn runtime_install_response(
    runtime_id: &str,
    job_id: &str,
    setup_choice: RuntimeSetupChoice,
    result: &desktoplab_runtime::RuntimeInstallExecutionResult,
) -> Value {
    json!({
        "source":"service_backed",
        "jobId":job_id,
        "runtimeId":runtime_id,
        "state":execution_state(result.state()),
        "verificationState":result.verification_state(),
        "retryClass":runtime_execution_retry_class(result.state()),
        "setupChoice":setup_choice.as_str(),
        "executionEvidence":result.evidence(),
        "blockedReason":if result.state() == RuntimeExecutionState::Blocked {json!(result.remediation())} else {Value::Null},
        "remediation":result.remediation()
    })
}

pub fn runtime_install_blocked_response(path: &str, error: RuntimeInstallError) -> Value {
    let reason = runtime_install_error_reason(&error);
    json!({
        "source":"service_backed",
        "jobId":Value::Null,
        "runtimeId":segment(path, 2),
        "state":"blocked",
        "verificationState":"pending",
        "retryClass":runtime_install_retry_class(&error),
        "blockedReason":reason
    })
}

pub fn runtime_install_error_blocked_reason(error: &RuntimeInstallError) -> &'static str {
    runtime_install_error_reason(error)
}
