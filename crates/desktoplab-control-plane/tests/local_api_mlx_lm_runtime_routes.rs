use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn runtime_inventory_lists_mlx_lm_as_desktoplab_managed_runtime() {
    let mut router = LocalApiRouter::default();
    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");

    let mlx_lm = inventory["runtimes"]
        .as_array()
        .expect("runtime inventory")
        .iter()
        .find(|runtime| runtime["runtimeId"] == "runtime.mlx-lm")
        .expect("MLX-LM runtime should be listed");

    assert_eq!(mlx_lm["displayName"], "MLX-LM Server");
    assert_eq!(mlx_lm["ownership"], "desktoplab_managed");
    assert_eq!(mlx_lm["provenance"]["runtimeId"], "runtime.mlx-lm");
    assert_eq!(mlx_lm["provenance"]["installSource"], "python_environment");
    assert_eq!(mlx_lm["provenance"]["integrity"]["state"], "unavailable");
    if host_supports_mlx_lm() {
        assert_eq!(mlx_lm["install"]["supported"], true);
    } else {
        assert_eq!(mlx_lm["install"]["supported"], false);
        assert_eq!(
            mlx_lm["install"]["blockedReason"],
            "Apple Silicon Mac required"
        );
    }
}

#[test]
fn mlx_lm_runtime_install_route_uses_python_environment_contract() {
    let mut router = LocalApiRouter::default();
    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.mlx-lm/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    assert_eq!(install["source"], "service_backed");
    assert_eq!(install["runtimeId"], "runtime.mlx-lm");
    if host_supports_mlx_lm() {
        assert_ne!(install["verificationState"], "pending");
    } else {
        assert_eq!(install["state"], "blocked");
        assert_eq!(install["verificationState"], "unsupported_platform");
        assert_eq!(
            install["blockedReason"],
            "MLX-LM Server is available only on Apple Silicon Macs."
        );
    }
}

#[test]
fn mlx_lm_runtime_route_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_mlx_lm_runtime_routes.rs",
        include_str!("local_api_mlx_lm_runtime_routes.rs"),
        90,
    )
    .expect("MLX-LM runtime route tests should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn host_supports_mlx_lm() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}
