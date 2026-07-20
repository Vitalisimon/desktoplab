use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn route_options_expose_selected_route_and_blocked_choices() {
    let mut router = LocalApiRouter::default();

    let options = route_json(&mut router, "GET", "/v1/routing/options", "");

    assert_eq!(options["selectedRouteId"], "route.local.unconfigured");
    assert_eq!(options["options"][0]["routeId"], "route.local.unconfigured");
    assert_eq!(options["options"][0]["status"], "unavailable");
    assert_eq!(
        options["options"][0]["disabledReason"],
        "runtime_and_model_not_verified"
    );
    assert_eq!(options["options"][1]["backendKind"], "cloud");
    assert_eq!(options["options"][1]["status"], "unavailable");
    assert_eq!(
        options["options"][1]["disabledReason"],
        "Connect OpenAI before routing work to the cloud."
    );
    assert_eq!(options["options"][2]["backendKind"], "external");
    assert_eq!(options["options"][2]["status"], "unavailable");
    assert_eq!(
        options["options"][2]["disabledReason"],
        "Connect the Codex bridge before routing work outside DesktopLab."
    );
}

#[test]
fn local_route_selection_requires_verified_runtime_and_model() {
    let mut router = LocalApiRouter::default();
    let response = router
        .route(
            "POST",
            "/v1/routing/options/selection",
            r#"{"routeId":"route.local.unconfigured"}"#,
        )
        .expect("selection route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("runtime_and_model_not_verified"));
}

#[test]
fn route_selection_is_backend_owned_and_reflected_in_workspace_route() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);

    let selected = route_json(
        &mut router,
        "POST",
        "/v1/routing/options/selection",
        r#"{"routeId":"route.local.gemma4-12b-q4"}"#,
    );
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(selected["selectedRouteId"], "route.local.gemma4-12b-q4");
    assert_eq!(workspace["route"]["routeId"], "route.local.gemma4-12b-q4");
    assert_eq!(workspace["route"]["modelDisplayName"], "Gemma 4 12B Q4");
}

#[test]
fn selected_local_route_uses_the_selected_candidate_metadata() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    post(
        &mut router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.qwen3.5-9b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.qwen3.5-9b-q4", "qwen3.5:9b");
    post(&mut router, "/v1/setup/complete", "{}");
    let _workspace = open_test_workspace(&mut router);

    let options = route_json(&mut router, "GET", "/v1/routing/options", "");
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(options["options"][0]["modelDisplayName"], "Qwen 3.5 9B Q4");
    assert_eq!(workspace["route"]["modelDisplayName"], "Qwen 3.5 9B Q4");
}

#[test]
fn blocked_catalog_models_cannot_be_selected_as_execution_routes() {
    let mut router = LocalApiRouter::default();
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.set_host_memory_gb_for_test(16);

    let response = router
        .route(
            "POST",
            "/v1/routing/options/selection",
            r#"{"routeId":"route.local.nemotron-70b-q4"}"#,
        )
        .expect("selection route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("not ready on this computer"));
}

#[test]
fn blocked_routes_cannot_be_selected_silently() {
    let mut router = LocalApiRouter::default();
    let response = router
        .route(
            "POST",
            "/v1/routing/options/selection",
            r#"{"routeId":"route.cloud.openai"}"#,
        )
        .expect("selection route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("Connect OpenAI"));

    let external = router
        .route(
            "POST",
            "/v1/routing/options/selection",
            r#"{"routeId":"route.external.codex"}"#,
        )
        .expect("selection route should exist");

    assert_eq!(external.status(), "400 Bad Request");
    assert!(external.body().contains("Connect the Codex bridge"));
}

#[test]
fn route_selection_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_route_selection.rs",
        include_str!("local_api_route_selection.rs"),
        190,
    )
    .expect("route selection test should stay focused");
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
    router.set_local_model_inventory_for_test(&["gemma4:12b"]);
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

fn open_test_workspace(router: &mut LocalApiRouter) -> TempDir {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let root = fixture.path().join("desktoplab");
    std::fs::create_dir(&root).expect("workspace should be created");
    post(
        router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    fixture
}
