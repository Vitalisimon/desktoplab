use desktoplab_backend_services::{
    BackendDiagnosticsService, DiagnosticServiceFamily, DiagnosticServiceState,
    DiagnosticsBundleGuard, DiagnosticsRepairPlanner, RepairActionFamily, RepairActionMode,
};
use xtask::check_logical_line_limit;

#[test]
fn diagnostics_repair_plans_redact_secrets_and_separate_actions_from_guidance() {
    let snapshot = BackendDiagnosticsService::new()
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Degraded("ollama stopped token=secret".into()),
        )
        .with_service(
            DiagnosticServiceFamily::Registry,
            DiagnosticServiceState::Degraded("offline api_key=sk-live-secret".into()),
        )
        .snapshot();

    let plan = DiagnosticsRepairPlanner::default()
        .with_executable(RepairActionFamily::Runtime)
        .plan(&snapshot);

    assert!(plan.has_action(RepairActionFamily::Runtime, RepairActionMode::Executable));
    assert!(plan.has_action(RepairActionFamily::Registry, RepairActionMode::GuidanceOnly));
    assert!(plan.summary().contains("[REDACTED]"));
    assert!(!plan.summary().contains("sk-live-secret"));
    assert!(!plan.summary().contains("token=secret"));
}

#[test]
fn diagnostics_repair_covers_runtime_model_provider_plugin_and_workspace_scan() {
    let snapshot = BackendDiagnosticsService::new()
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Degraded("runtime_missing".into()),
        )
        .with_service(
            DiagnosticServiceFamily::Model,
            DiagnosticServiceState::Degraded("model_missing".into()),
        )
        .with_service(
            DiagnosticServiceFamily::Provider,
            DiagnosticServiceState::Degraded("provider_credentials_missing".into()),
        )
        .with_service(
            DiagnosticServiceFamily::Plugin,
            DiagnosticServiceState::Degraded("unverified_plugin".into()),
        )
        .with_service(
            DiagnosticServiceFamily::WorkspaceScan,
            DiagnosticServiceState::Degraded("workspace_scan_file_limit_exceeded".into()),
        )
        .snapshot();

    let plan = DiagnosticsRepairPlanner::all_executable().plan(&snapshot);

    for family in [
        RepairActionFamily::Runtime,
        RepairActionFamily::Model,
        RepairActionFamily::Provider,
        RepairActionFamily::Plugin,
        RepairActionFamily::WorkspaceScan,
    ] {
        assert!(plan.has_action(family, RepairActionMode::Executable));
    }
}

#[test]
fn runtime_install_failures_are_retryable_but_os_level_repairs_are_guidance_only() {
    let snapshot = BackendDiagnosticsService::new()
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Degraded("runtime_install_failed checksum mismatch".into()),
        )
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Degraded(
                "os_level_repair_unsupported driver reinstall required".into(),
            ),
        )
        .snapshot();

    let plan = DiagnosticsRepairPlanner::default()
        .with_executable(RepairActionFamily::Runtime)
        .plan(&snapshot);

    assert!(
        plan.summary()
            .contains("runtime_install_failed checksum mismatch")
    );
    assert!(plan.has_action(RepairActionFamily::Runtime, RepairActionMode::Executable));
    assert!(
        plan.summary()
            .contains("os_level_repair_unsupported driver reinstall required")
    );
    assert!(plan.has_action(RepairActionFamily::Runtime, RepairActionMode::GuidanceOnly));
}

#[test]
fn diagnostics_bundle_guard_caps_productization_bundle_size() {
    let guard = DiagnosticsBundleGuard::new(12);

    assert!(guard.check("small").is_ok());
    assert_eq!(
        guard.check("0123456789abcdef").unwrap_err(),
        "diagnostics_bundle_too_large"
    );
}

#[test]
fn diagnostics_repair_sources_stay_below_line_count_guards() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backend-services/src/diagnostics_repair.rs",
            include_str!("../src/diagnostics_repair.rs"),
            280,
        ),
        (
            "crates/desktoplab-backend-services/src/performance.rs",
            include_str!("../src/performance.rs"),
            220,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("diagnostics productization modules should stay focused");
    }
}
