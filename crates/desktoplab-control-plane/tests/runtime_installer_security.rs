use desktoplab_runtime::{
    InstallHostCapacity, InstallPlan, InstallerSource, ProcessCommand, RuntimeId,
    RuntimeInstallError, RuntimeInstallExecutionStrategy, RuntimeInstallPlanner,
    RuntimeInstallRequest,
};

#[test]
fn unsafe_installer_source_is_rejected_before_runtime_install_job_creation() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let plan = verified_plan("file:///tmp/ollama-install.sh");

    let error = planner
        .plan_job(RuntimeInstallRequest::new(plan).with_setup_plan_accepted(true))
        .expect_err("file installer sources must not create executable install jobs");

    assert_eq!(error, RuntimeInstallError::UnsafeInstallerSource);
}

#[test]
fn unverified_runtime_download_cannot_create_runtime_install_job() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let plan = InstallPlan::new(RuntimeId::new("runtime.custom"), "Custom runtime")
        .with_target_platform("linux-x64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_installer_source(InstallerSource::signed_url(
            "https://desktoplab.test/custom.sh",
            "",
        ));

    let error = planner
        .plan_job(RuntimeInstallRequest::new(plan).with_setup_plan_accepted(true))
        .expect_err("install jobs require verification metadata");

    assert_eq!(error, RuntimeInstallError::MissingVerificationMetadata);
}

#[test]
fn install_command_evidence_redacts_secret_bearing_arguments() {
    let evidence = ProcessCommand::new("curl")
        .arg("--fail")
        .arg("https://registry.desktoplab.test/runtime.sh?api_key=raw-secret")
        .arg("token=raw-token")
        .evidence();

    assert!(evidence.contains("[REDACTED]"));
    assert!(!evidence.contains("raw-secret"));
    assert!(!evidence.contains("raw-token"));
}

fn verified_plan(url: &str) -> InstallPlan {
    InstallPlan::new(RuntimeId::new("runtime.ollama"), "Ollama")
        .with_target_platform("linux-x64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_disk_requirement_gb(4)
        .with_network_required(true)
        .with_installer_source(InstallerSource::signed_url_with_signature(
            url,
            "sha256:expected",
            "sig:vendor",
        ))
        .with_verification_step("checksum sha256:expected")
}
