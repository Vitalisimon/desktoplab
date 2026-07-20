use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn setup_preview_uses_host_profile_instead_of_static_hardware() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");

    assert_eq!(preview["source"], "service_backed");
    assert_eq!(
        preview["hardware"]["operatingSystem"]["value"],
        std::env::consts::OS
    );
    assert_eq!(
        preview["hardware"]["architecture"]["value"],
        std::env::consts::ARCH
    );
    assert_eq!(preview["hardware"]["vramGb"]["value"], Value::Null);
    assert_eq!(preview["hardware"]["vramGb"]["confidence"], "unknown");
    assert_eq!(
        preview["hardware"]["acceleratorKind"]["label"],
        "Accelerator type"
    );
    assert_eq!(preview["highEndLocal"]["source"], "hardware_probe");
    assert!(preview["highEndLocal"]["runtimeChoices"].is_array());
    assert_eq!(
        preview["highEndLocal"]["claimState"],
        "certification_required"
    );
}

#[test]
fn setup_preview_warnings_match_the_measured_host_facts() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");
    let warnings = preview["warnings"].as_array().expect("warnings");
    let has_warning = |code: &str| warnings.iter().any(|warning| warning == code);
    let hardware = &preview["hardware"];

    if hardware["gpu"]["confidence"] == "confirmed" {
        assert!(!has_warning("gpu_probe_unavailable"));
    }
    if hardware["ramGb"]["confidence"] == "confirmed" {
        let ram = hardware["ramGb"]["value"].as_u64().expect("RAM value");
        assert_eq!(has_warning("limited_memory"), ram <= 8);
    }
    if hardware["storageAvailableGb"]["confidence"] == "confirmed" {
        let storage = hardware["storageAvailableGb"]["value"]
            .as_u64()
            .expect("storage value");
        assert_eq!(has_warning("low_storage"), storage < 64);
    }
}

#[test]
fn setup_preview_marks_lm_studio_as_external_guided() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");

    let lm_studio = preview["runtimeRecommendations"]
        .as_array()
        .expect("runtime recommendations")
        .iter()
        .find(|runtime| runtime["manifestId"] == "runtime.lm-studio")
        .expect("LM Studio runtime should be present");

    assert_eq!(lm_studio["installMode"], "external_guided");
}

#[test]
fn setup_preview_exposes_mlx_lm_as_macos_native_runtime() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");

    let runtime_recommendations = preview["runtimeRecommendations"]
        .as_array()
        .expect("runtime recommendations");
    let mlx_lm = runtime_recommendations
        .iter()
        .find(|runtime| runtime["manifestId"] == "runtime.mlx-lm");

    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        let mlx_lm = mlx_lm.expect("MLX-LM runtime should be present on Apple Silicon");
        assert_eq!(mlx_lm["displayName"], "MLX-LM Server");
        assert_eq!(mlx_lm["installMode"], "python_environment");
    } else {
        assert!(
            mlx_lm.is_none(),
            "MLX-LM should not be proposed as setup runtime on unsupported hosts"
        );
    }
}

#[test]
fn local_api_setup_preview_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_setup_preview.rs",
        include_str!("local_api_setup_preview.rs"),
        120,
    )
    .expect("setup preview test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
