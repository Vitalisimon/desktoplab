use desktoplab_runtime::{
    InstallHostCapacity, InstallPlan, InstallerSource, RuntimeId, RuntimeInstallApproval,
    RuntimeInstallError, RuntimeInstallExecutionStrategy, RuntimeInstallManagement,
    RuntimeInstallPlanner, RuntimeInstallRequest, RuntimeInstallStatus,
};
use xtask::check_logical_line_limit;

#[test]
fn install_cannot_start_without_accepted_setup_plan() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let request = RuntimeInstallRequest::new(ollama_plan()).with_setup_plan_accepted(false);

    let error = planner
        .plan_job(request)
        .expect_err("runtime install needs accepted setup plan");

    assert_eq!(error, RuntimeInstallError::SetupPlanNotAccepted);
}

#[test]
fn insufficient_disk_blocks_install() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(2, true));
    let request = RuntimeInstallRequest::new(ollama_plan()).with_setup_plan_accepted(true);

    let error = planner
        .plan_job(request)
        .expect_err("insufficient disk should block install");

    assert_eq!(
        error,
        RuntimeInstallError::InsufficientDisk {
            required_gb: 8,
            available_gb: 2,
        }
    );
}

#[test]
fn runtime_install_remains_automatic_after_accepted_setup_plan() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let request = RuntimeInstallRequest::new(ollama_plan()).with_setup_plan_accepted(true);

    let job = planner
        .plan_job(request)
        .expect("install should be planned");

    assert_eq!(job.status(), RuntimeInstallStatus::Queued);
    assert_eq!(
        job.approval(),
        RuntimeInstallApproval::AutomaticAfterSetupAcceptance
    );
}

#[test]
fn plan_explains_steps_requirements_source_and_verification() {
    let explanation = ollama_plan().explanation();

    assert!(explanation.contains("download Ollama installer on demand"));
    assert!(explanation.contains("requires: disk.available_gb >= 8"));
    assert!(explanation.contains("installer_source: https://desktoplab.test/ollama"));
    assert!(explanation.contains("verify: checksum sha256:test"));
}

#[test]
fn install_plan_carries_executable_adapter_contract() {
    let plan = ollama_plan();
    let source = plan
        .installer_source()
        .expect("installer source should be explicit");

    assert_eq!(plan.target_platform(), Some("darwin-arm64"));
    assert_eq!(
        plan.execution_strategy(),
        Some(RuntimeInstallExecutionStrategy::NativeInstaller)
    );
    assert_eq!(plan.disk_requirement_gb(), Some(8));
    assert_eq!(source.url(), "https://desktoplab.test/ollama");
    assert_eq!(source.checksum(), "sha256:test");
    assert_eq!(source.signature(), Some("sig:test"));
}

#[test]
fn install_execution_refuses_plans_without_verification_metadata() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let request = RuntimeInstallRequest::new(
        InstallPlan::new(RuntimeId::new("runtime.custom"), "Custom runtime")
            .with_step("download installer")
            .with_target_platform("darwin-arm64")
            .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
            .with_disk_requirement_gb(4)
            .with_network_required(true),
    )
    .with_setup_plan_accepted(true);

    let error = planner
        .plan_job(request)
        .expect_err("unverified installers must not be executable");

    assert_eq!(error, RuntimeInstallError::MissingVerificationMetadata);
}

#[test]
fn externally_managed_runtime_cannot_be_silently_overwritten() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let request = RuntimeInstallRequest::new(
        ollama_plan().with_management(RuntimeInstallManagement::ExternallyManaged),
    )
    .with_setup_plan_accepted(true);

    let error = planner
        .plan_job(request)
        .expect_err("externally managed runtimes require explicit non-overwrite handling");

    assert_eq!(error, RuntimeInstallError::ExternallyManagedRuntime);
}

#[test]
fn runtime_installer_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/install.rs",
        include_str!("../src/install.rs"),
        280,
    )
    .expect("runtime install source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/install_contract.rs",
        include_str!("../src/install_contract.rs"),
        160,
    )
    .expect("runtime install contract source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/install_job.rs",
        include_str!("../src/install_job.rs"),
        180,
    )
    .expect("runtime install job source should stay below the initial line-count guard");
}

fn ollama_plan() -> InstallPlan {
    InstallPlan::new(RuntimeId::new("runtime.ollama"), "Ollama")
        .with_step("download Ollama installer on demand")
        .with_step("install runtime")
        .with_target_platform("darwin-arm64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_disk_requirement_gb(8)
        .with_network_required(true)
        .with_installer_source(InstallerSource::signed_url_with_signature(
            "https://desktoplab.test/ollama",
            "sha256:test",
            "sig:test",
        ))
        .with_verification_step("checksum sha256:test")
        .with_verification_step("runtime health")
}
