use desktoplab_packaging::{PackagingSmokeResult, PlatformTarget, SmokeState};

#[test]
fn smoke_result_records_every_required_state() {
    let result = PackagingSmokeResult::new(
        PlatformTarget::MacosAarch64,
        "DesktopLab.app",
        SmokeState::Passed,
        SmokeState::Passed,
        SmokeState::Passed,
        SmokeState::Passed,
    );

    assert_eq!(result.platform(), PlatformTarget::MacosAarch64);
    assert_eq!(result.artifact(), "DesktopLab.app");
    assert_eq!(result.install_state(), SmokeState::Passed);
    assert_eq!(result.launch_state(), SmokeState::Passed);
    assert_eq!(result.local_api_state(), SmokeState::Passed);
    assert_eq!(result.cleanup_state(), SmokeState::Passed);
}

#[test]
fn unsupported_platform_smoke_is_not_run_not_passed() {
    let result = PackagingSmokeResult::unsupported(PlatformTarget::LinuxX64, "DesktopLab.AppImage");

    assert_eq!(result.install_state(), SmokeState::NotRun);
    assert_eq!(result.launch_state(), SmokeState::NotRun);
    assert_eq!(result.local_api_state(), SmokeState::NotRun);
    assert_eq!(result.cleanup_state(), SmokeState::NotRun);
    assert_ne!(result.install_state(), SmokeState::Passed);
}

#[test]
fn smoke_result_schema_has_stable_json_keys() {
    let result = PackagingSmokeResult::unsupported(PlatformTarget::WindowsX64, "DesktopLab.exe");
    let json = result.to_json_line();

    assert!(json.contains("\"platform\":\"windows-x64\""));
    assert!(json.contains("\"artifact\":\"DesktopLab.exe\""));
    assert!(json.contains("\"installState\":\"not_run\""));
    assert!(json.contains("\"launchState\":\"not_run\""));
    assert!(json.contains("\"localApiState\":\"not_run\""));
    assert!(json.contains("\"cleanupState\":\"not_run\""));
}

#[test]
fn packaging_smoke_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-packaging/tests/packaging_smoke.rs",
        include_str!("packaging_smoke.rs"),
        120,
    )
    .expect("packaging smoke test should stay focused");
}
