use desktoplab_control_plane::{
    ApiSurface, ClientKind, ControlPlane, ControlPlaneError, ControlPlaneStatus, ErrorCode,
    LifecycleState, ReadinessState, ShutdownMode, VersionInfo,
};
use xtask::check_logical_line_limit;

#[test]
fn new_control_plane_reports_started_health_and_not_ready_until_marked_ready() {
    let mut control_plane = ControlPlane::new(VersionInfo::new("0.1.0", "v1"));

    assert_eq!(control_plane.lifecycle_state(), LifecycleState::Initialized);
    assert_eq!(control_plane.health().status(), ControlPlaneStatus::Healthy);
    assert_eq!(control_plane.readiness().state(), ReadinessState::Starting);

    control_plane.mark_ready();

    assert_eq!(control_plane.readiness().state(), ReadinessState::Ready);
}

#[test]
fn version_endpoint_contract_is_stable_and_api_surface_is_v1() {
    let control_plane = ControlPlane::new(VersionInfo::new("0.1.0", "v1"));
    let version = control_plane.version();
    let api = control_plane.api_surface();

    assert_eq!(version.product_version(), "0.1.0");
    assert_eq!(version.api_version(), "v1");
    assert_eq!(api.base_path(), "/v1");
    assert_eq!(api.health_path(), "/health");
    assert_eq!(api.readiness_path(), "/v1/readiness");
    assert_eq!(api.version_path(), "/v1/version");
}

#[test]
fn structured_errors_have_stable_machine_codes_and_messages() {
    let error = ControlPlaneError::unauthorized("local auth token is missing");

    assert_eq!(error.code(), ErrorCode::Unauthorized);
    assert_eq!(error.message(), "local auth token is missing");
    assert_eq!(error.http_status(), 401);
    assert_eq!(
        error.to_string(),
        "UNAUTHORIZED: local auth token is missing"
    );
}

#[test]
fn shutdown_request_changes_lifecycle_without_losing_health_contract() {
    let mut control_plane = ControlPlane::new(VersionInfo::new("0.1.0", "v1"));

    control_plane.request_shutdown(ShutdownMode::Graceful);

    assert_eq!(control_plane.lifecycle_state(), LifecycleState::Stopping);
    assert_eq!(control_plane.shutdown_mode(), Some(ShutdownMode::Graceful));
    assert_eq!(
        control_plane.health().status(),
        ControlPlaneStatus::Draining
    );
}

#[test]
fn client_contract_is_not_desktop_ui_specific() {
    assert!(ClientKind::all_supported().contains(&ClientKind::DesktopShell));
    assert!(ClientKind::all_supported().contains(&ClientKind::DeveloperCli));
    assert!(ClientKind::all_supported().contains(&ClientKind::Diagnostics));
    assert!(ClientKind::all_supported().contains(&ClientKind::FutureCompanion));

    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("tauri"));
    assert!(!manifest.contains("electron"));
    assert!(!manifest.contains("wry"));
}

#[test]
fn control_plane_files_stay_below_initial_line_count_guard() {
    let lib = include_str!("../src/lib.rs");

    check_logical_line_limit("crates/desktoplab-control-plane/src/lib.rs", lib, 250)
        .expect("control-plane lib should stay below the initial line-count guard");
}

#[test]
fn api_surface_type_is_exported_for_future_http_adapter() {
    let api = ApiSurface::v1();

    assert_eq!(api.base_path(), "/v1");
}
