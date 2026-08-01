use std::path::PathBuf;

use super::helpers::{bool_body_field, host_target, number_body_field, string_body_field};

pub(super) fn execute(body: &str) -> desktoplab_runtime::RuntimeInstallExecutionResult {
    let Ok(plan) = managed_plan(body) else {
        return desktoplab_runtime::RuntimeInstallExecutionResult::blocked_with_state(
            "managed_plan_rejected",
            "lm-studio managed plan rejected",
            "Accept the vendor terms and use the exact supported DesktopLab runtime plan.",
        );
    };
    desktoplab_runtime::execute_managed_lm_studio(
        &desktoplab_runtime::SystemProcessRunner,
        &desktoplab_runtime::HttpLmStudioModelProbe,
        &plan,
    )
}

pub(super) fn verify_existing() -> desktoplab_runtime::RuntimeInstallExecutionResult {
    let discovery = desktoplab_runtime::discover_system_lm_studio();
    if discovery.ready() {
        return desktoplab_runtime::RuntimeInstallExecutionResult::verified_existing(
            discovery.evidence(),
        );
    }
    desktoplab_runtime::RuntimeInstallExecutionResult::blocked_with_state(
        discovery.state().as_str(),
        discovery.evidence(),
        discovery.remediation(),
    )
}

pub(super) fn managed_plan(
    body: &str,
) -> Result<desktoplab_runtime::LmStudioManagedPlan, desktoplab_runtime::LmStudioManagedPlanError> {
    desktoplab_runtime::LmStudioManagedPlan::new(
        managed_root(),
        &host_target(),
        string_body_field(body, "modelRef").unwrap_or_else(|| "openai/gpt-oss-20b".to_string()),
        string_body_field(body, "apiModelId")
            .unwrap_or_else(|| "desktoplab-gpt-oss-20b".to_string()),
        number_body_field(body, "port")
            .and_then(|port| u16::try_from(port).ok())
            .unwrap_or(12_345),
        bool_body_field(body, "vendorTermsAccepted").unwrap_or(false),
    )
}

pub(super) fn status() -> Option<desktoplab_runtime::LmStudioManagedStatus> {
    let plan = managed_plan(r#"{"vendorTermsAccepted":true}"#).ok()?;
    desktoplab_runtime::inspect_managed_lm_studio(
        &desktoplab_runtime::SystemProcessRunner,
        &desktoplab_runtime::HttpLmStudioModelProbe,
        &plan,
    )
}

pub(crate) fn models() -> Vec<String> {
    status()
        .filter(desktoplab_runtime::LmStudioManagedStatus::ready)
        .map(|status| status.models().to_vec())
        .unwrap_or_default()
}

pub(crate) fn connection() -> Option<(String, Vec<String>)> {
    status()
        .filter(desktoplab_runtime::LmStudioManagedStatus::ready)
        .map(|status| (status.endpoint().to_string(), status.models().to_vec()))
}

pub(crate) fn capability_connection() -> Option<(String, Vec<String>, String)> {
    status()
        .filter(desktoplab_runtime::LmStudioManagedStatus::ready)
        .map(|status| {
            (
                status.endpoint().to_string(),
                status.models().to_vec(),
                status.version().to_string(),
            )
        })
}

fn managed_root() -> PathBuf {
    if let Some(root) = std::env::var_os("DESKTOPLAB_RUNTIME_DATA_DIR") {
        return PathBuf::from(root)
            .join("DesktopLab")
            .join("runtimes")
            .join("lm-studio");
    }
    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library")
            .join("Application Support")
            .join("DesktopLab")
            .join("runtimes")
            .join("lm-studio");
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("DesktopLab")
            .join("runtimes")
            .join("lm-studio");
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("share"))
        })
        .unwrap_or_else(std::env::temp_dir)
        .join("DesktopLab")
        .join("runtimes")
        .join("lm-studio")
}

#[cfg(test)]
mod tests {
    use super::managed_plan;

    #[test]
    fn managed_plan_requires_explicit_vendor_terms_acceptance() {
        assert_eq!(
            managed_plan(r#"{"vendorTermsAccepted":false}"#),
            Err(desktoplab_runtime::LmStudioManagedPlanError::VendorTermsNotAccepted)
        );
    }

    #[test]
    fn managed_plan_uses_pinned_default_model_and_loopback_port() {
        let plan = managed_plan(r#"{"vendorTermsAccepted":true}"#).expect("supported host");
        assert_eq!(plan.model_id(), "openai/gpt-oss-20b");
        assert_eq!(plan.api_model_id(), "desktoplab-gpt-oss-20b");
        assert_eq!(plan.endpoint(), "http://127.0.0.1:12345");
    }
}
