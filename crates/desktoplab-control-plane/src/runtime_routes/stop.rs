use desktoplab_runtime::{
    RuntimeExecutionState, RuntimeInstallExecutionResult, SystemProcessRunner,
};
use serde_json::{Value, json};

#[must_use]
pub(crate) fn execute(runtime_id: &str) -> RuntimeInstallExecutionResult {
    match runtime_id {
        "runtime.lm-studio" => stop_lm_studio(),
        "runtime.mlx-lm" => stop_mlx_lm(),
        _ => RuntimeInstallExecutionResult::blocked_with_state(
            "stop_not_supported",
            "runtime does not expose DesktopLab-managed ownership evidence",
            "DesktopLab only stops ownership-verified managed LM Studio and MLX-LM runtimes.",
        ),
    }
}

#[must_use]
pub(crate) fn response(runtime_id: &str, result: &RuntimeInstallExecutionResult) -> Value {
    json!({
        "source":"service_backed",
        "runtimeId":runtime_id,
        "state":state(result.state()),
        "verificationState":if result.state() == RuntimeExecutionState::Completed {"stopped"} else {result.verification_state()},
        "retryClass":"non_retryable",
        "executionEvidence":result.evidence(),
        "blockedReason":if result.state() == RuntimeExecutionState::Blocked {json!(result.remediation())} else {Value::Null},
        "remediation":result.remediation()
    })
}

fn stop_lm_studio() -> RuntimeInstallExecutionResult {
    let Ok(plan) = super::lm_studio_managed::managed_plan(r#"{"vendorTermsAccepted":true}"#) else {
        return unsupported_host("LM Studio");
    };
    desktoplab_runtime::stop_managed_lm_studio(&SystemProcessRunner, &plan)
}

fn stop_mlx_lm() -> RuntimeInstallExecutionResult {
    let Ok(plan) = super::mlx_lm_managed::managed_plan(r#"{"modelLicenseAccepted":true}"#) else {
        return unsupported_host("MLX-LM");
    };
    desktoplab_runtime::stop_managed_mlx_lm(&SystemProcessRunner, &plan)
}

fn unsupported_host(runtime: &str) -> RuntimeInstallExecutionResult {
    RuntimeInstallExecutionResult::blocked_with_state(
        "managed_plan_unavailable",
        format!("{runtime} managed runtime plan is unavailable on this host"),
        "DesktopLab cannot resolve a supported ownership marker for this runtime on this host.",
    )
}

fn state(state: RuntimeExecutionState) -> &'static str {
    match state {
        RuntimeExecutionState::Completed => "completed",
        RuntimeExecutionState::Blocked => "blocked",
        RuntimeExecutionState::ExternalGuided => "external_guided",
        RuntimeExecutionState::Failed => "failed",
    }
}
