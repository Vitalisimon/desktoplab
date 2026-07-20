use desktoplab_runtime::{
    InstallHostCapacity, InstallPlan, InstallerSource, RuntimeCachedInstallerArtifact,
    RuntimeCommand, RuntimeDownloadFailure, RuntimeDownloadFailureKind, RuntimeDownloadRetryClass,
    RuntimeDownloadVerification, RuntimeId, RuntimeInstallError, RuntimeInstallExecutionStrategy,
    RuntimeInstallPlanner, RuntimeInstallRequest, RuntimeManager, RuntimeState,
};
use xtask::check_logical_line_limit;

#[test]
fn runtime_source_failures_are_retryable_or_blocked_by_error_class() {
    assert_eq!(
        RuntimeDownloadFailure::new(RuntimeDownloadFailureKind::SourceUnavailable).retry_class(),
        RuntimeDownloadRetryClass::Retryable
    );
    assert_eq!(
        RuntimeDownloadFailure::new(RuntimeDownloadFailureKind::ChecksumMismatch).retry_class(),
        RuntimeDownloadRetryClass::Blocked
    );
}

#[test]
fn checksum_mismatch_blocks_install_completion() {
    let verification = RuntimeDownloadVerification::new("sha256:expected", "sha256:actual");

    let failure = verification
        .verify()
        .expect_err("checksum mismatch should fail verification");

    assert_eq!(failure.kind(), RuntimeDownloadFailureKind::ChecksumMismatch);
    assert_eq!(failure.retry_class(), RuntimeDownloadRetryClass::Blocked);
}

#[test]
fn checksum_match_allows_download_to_feed_install() {
    let verification = RuntimeDownloadVerification::new("sha256:expected", "sha256:expected");

    verification
        .verify()
        .expect("matching checksum should allow install completion");
}

#[test]
fn install_execution_still_requires_accepted_setup_plan() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));

    let error = planner
        .plan_job(RuntimeInstallRequest::new(verified_plan()))
        .expect_err("setup acceptance is required before install execution");

    assert_eq!(error, RuntimeInstallError::SetupPlanNotAccepted);
}

#[test]
fn unverified_runtime_download_cannot_complete_install() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let plan = InstallPlan::new(RuntimeId::new("runtime.custom"), "Custom runtime")
        .with_target_platform("darwin-arm64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller);

    let error = planner
        .plan_job(RuntimeInstallRequest::new(plan).with_setup_plan_accepted(true))
        .expect_err("unverified plan should not execute");

    assert_eq!(error, RuntimeInstallError::MissingVerificationMetadata);
}

#[test]
fn offline_runtime_install_requires_verified_cached_artifact() {
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, false));

    let error = planner
        .plan_job(RuntimeInstallRequest::new(verified_plan()).with_setup_plan_accepted(true))
        .expect_err("offline install without cached verified artifact should be blocked");

    assert_eq!(error, RuntimeInstallError::NetworkUnavailable);

    let cached_job = planner
        .plan_job(
            RuntimeInstallRequest::new(verified_plan().with_cached_artifact(
                RuntimeCachedInstallerArtifact::verified(
                    "cache://runtime.ollama/macos-arm64",
                    "sha256:expected",
                ),
            ))
            .with_setup_plan_accepted(true),
        )
        .expect("offline install may continue with a verified cached artifact");

    assert!(
        cached_job
            .plan_preview()
            .contains("cached_installer: verified")
    );
}

#[test]
fn completed_runtime_install_updates_inventory() {
    let runtime_id = RuntimeId::new("runtime.ollama");
    let mut manager = RuntimeManager::new();
    manager.register_runtime(runtime_id.clone(), "Ollama");
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));
    let job = planner
        .plan_job(RuntimeInstallRequest::new(verified_plan()).with_setup_plan_accepted(true))
        .expect("verified install should create a job");

    job.complete_install(&mut manager, "0.5.0");

    assert_eq!(manager.status(&runtime_id).state(), RuntimeState::Installed);
    assert_eq!(
        manager.audit_log(),
        &[RuntimeCommand::mark_installed(runtime_id, "0.5.0")]
    );
}

#[test]
fn runtime_download_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/download.rs",
        include_str!("../src/download.rs"),
        180,
    )
    .expect("runtime download source should stay below the initial line-count guard");
}

fn verified_plan() -> InstallPlan {
    InstallPlan::new(RuntimeId::new("runtime.ollama"), "Ollama")
        .with_target_platform("darwin-arm64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_disk_requirement_gb(4)
        .with_network_required(true)
        .with_installer_source(InstallerSource::signed_url_with_signature(
            "https://ollama.com/download",
            "sha256:expected",
            "sig:vendor",
        ))
        .with_verification_step("checksum sha256:expected")
}
