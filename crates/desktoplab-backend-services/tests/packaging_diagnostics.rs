use desktoplab_backend_services::{BackendDiagnosticsService, PackagingDiagnostics};

#[test]
fn packaging_diagnostics_include_release_context_without_secrets() {
    let snapshot = BackendDiagnosticsService::new()
        .with_packaging_diagnostics(PackagingDiagnostics::new(
            "0.1.0",
            "dev",
            "macos-aarch64",
            "updates_disabled",
            "token=secret signing_identity=DeveloperID local_path=/Users/example/DesktopLab",
        ))
        .snapshot();

    let bundle = snapshot.bundle();

    assert!(bundle.contains("app_version=0.1.0"));
    assert!(bundle.contains("package_channel=dev"));
    assert!(bundle.contains("artifact_target=macos-aarch64"));
    assert!(bundle.contains("update_state=updates_disabled"));
    assert!(bundle.contains("support_evidence=local_only"));
    assert!(!bundle.contains("secret"));
    assert!(!bundle.contains("DeveloperID"));
    assert!(!bundle.contains("/Users/example"));
}

#[test]
fn packaging_diagnostics_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/packaging_diagnostics.rs",
        include_str!("packaging_diagnostics.rs"),
        100,
    )
    .expect("packaging diagnostics test should stay focused");
}
