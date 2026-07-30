use std::path::PathBuf;

use super::helpers::{bool_body_field, host_target, number_body_field};

const MODEL_ID: &str = "mlx-community/SmolLM3-3B-4bit";
const MODEL_REVISION: &str = "d3a7e0594d6642dbcfb7d149bed8b0bdf49f95ce";
const MODEL_STORAGE_BYTES: u64 = 1_747_260_812;

pub(super) fn execute(body: &str) -> desktoplab_runtime::RuntimeInstallExecutionResult {
    let Ok(plan) = managed_plan(body) else {
        return desktoplab_runtime::RuntimeInstallExecutionResult::blocked_with_state(
            "managed_plan_rejected",
            "mlx-lm managed plan rejected",
            "Accept the Apache-2.0 model license and use the exact Apple Silicon plan.",
        );
    };
    desktoplab_runtime::execute_managed_mlx_lm(
        &desktoplab_runtime::SystemProcessRunner,
        &desktoplab_runtime::SystemProcessSpawner,
        &desktoplab_runtime::HttpLmStudioModelProbe,
        &plan,
    )
}

pub(super) fn verify() -> desktoplab_runtime::RuntimeInstallExecutionResult {
    let Some(status) = status() else {
        return desktoplab_runtime::RuntimeInstallExecutionResult::blocked_with_state(
            "managed_runtime_missing",
            "mlx-lm ownership evidence unavailable",
            "Create the DesktopLab-managed MLX-LM environment first.",
        );
    };
    if status.ready() {
        return desktoplab_runtime::RuntimeInstallExecutionResult::verified_existing(format!(
            "managed MLX-LM ready; endpoint={}; revision={}",
            status.endpoint(),
            status.model_revision()
        ));
    }
    desktoplab_runtime::RuntimeInstallExecutionResult::blocked_with_state(
        "managed_runtime_degraded",
        "mlx-lm managed runtime is not ready",
        "Repair the DesktopLab-managed MLX-LM environment.",
    )
}

pub(crate) fn status() -> Option<desktoplab_runtime::MlxLmManagedStatus> {
    let plan = managed_plan(r#"{"modelLicenseAccepted":true}"#).ok()?;
    desktoplab_runtime::inspect_managed_mlx_lm(
        &desktoplab_runtime::SystemProcessRunner,
        &desktoplab_runtime::HttpLmStudioModelProbe,
        &plan,
    )
}

pub(super) fn managed_plan(
    body: &str,
) -> Result<desktoplab_runtime::MlxLmManagedPlan, desktoplab_runtime::MlxLmManagedPlanError> {
    desktoplab_runtime::MlxLmManagedPlan::new(
        managed_root(),
        &host_target(),
        MODEL_ID,
        MODEL_REVISION,
        "apache-2.0",
        MODEL_STORAGE_BYTES,
        number_body_field(body, "port")
            .and_then(|port| u16::try_from(port).ok())
            .unwrap_or(18_080),
        bool_body_field(body, "modelLicenseAccepted").unwrap_or(false),
    )
}

fn managed_root() -> PathBuf {
    if let Some(root) = std::env::var_os("DESKTOPLAB_RUNTIME_DATA_DIR") {
        return PathBuf::from(root)
            .join("DesktopLab")
            .join("runtimes")
            .join("mlx-lm");
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library")
        .join("Application Support")
        .join("DesktopLab")
        .join("runtimes")
        .join("mlx-lm")
}

#[cfg(test)]
mod tests {
    use super::managed_plan;

    #[test]
    fn managed_plan_requires_license_acceptance_and_exact_model() {
        assert_eq!(
            managed_plan(r#"{"modelLicenseAccepted":false}"#),
            Err(desktoplab_runtime::MlxLmManagedPlanError::ModelLicenseNotAccepted)
        );
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            let plan =
                managed_plan(r#"{"modelLicenseAccepted":true}"#).expect("supported Apple host");
            assert_eq!(plan.model_id(), "mlx-community/SmolLM3-3B-4bit");
            assert_eq!(
                plan.model_revision(),
                "d3a7e0594d6642dbcfb7d149bed8b0bdf49f95ce"
            );
            assert_eq!(plan.endpoint(), "http://127.0.0.1:18080");
        }
    }
}
