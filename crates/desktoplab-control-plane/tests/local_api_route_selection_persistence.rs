use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn route_selection_persists_across_router_reopen() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
        .expect("router should open");
    mark_setup_ready(&mut router);
    open_test_workspace(&mut router, fixture.path());
    router.set_local_model_inventory_for_test(&["gemma4:12b", "qwen3.5:9b"]);
    router.mark_model_verified_for_test("runtime.ollama", "model.qwen3.5-9b-q4", "qwen3.5:9b");

    let selected = route_json(
        &mut router,
        "POST",
        "/v1/routing/options/selection",
        r#"{"routeId":"route.local.qwen3.5-9b-q4"}"#,
    );
    assert_eq!(selected["selectedRouteId"], "route.local.qwen3.5-9b-q4");

    let mut reopened = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
        .expect("router should reopen");
    let workspace = route_json(&mut reopened, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace["route"]["routeId"], "route.local.qwen3.5-9b-q4");
    assert_eq!(workspace["route"]["modelDisplayName"], "Qwen 3.5 9B Q4");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn uncertified_mlx_route_cannot_be_selected_or_persisted() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(64);
    let response = router
        .route(
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.mlx-lm","modelId":"model.mlx-qwen-3.5-4b-8bit"}"#,
        )
        .unwrap();
    let payload: Value = serde_json::from_str(response.body()).unwrap();

    assert_eq!(response.status(), "400 Bad Request");
    assert_eq!(payload["code"], "SETUP_SELECTION_INCOMPATIBLE");
}

#[test]
fn route_selection_persistence_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_route_selection_persistence.rs",
        include_str!("local_api_route_selection_persistence.rs"),
        145,
    )
    .expect("route selection persistence test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}

fn open_test_workspace(router: &mut LocalApiRouter, root: &std::path::Path) {
    let workspace = root.join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should be created");
    post(
        router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace),
    );
}
