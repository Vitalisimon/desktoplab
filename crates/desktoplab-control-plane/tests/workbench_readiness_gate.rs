use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workbench_route_blocks_until_setup_and_model_are_ready() {
    let mut router = LocalApiRouter::default();

    let blocked = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(blocked["route"]["status"], "blocked");
    assert_eq!(blocked["session"], Value::Null);
    assert_eq!(blocked["route"]["nextAction"], "complete_setup");
    assert_eq!(blocked["route"]["nextActionLabel"], "Finish setup");
}

#[test]
fn prompt_start_is_rejected_before_backend_readiness() {
    let mut router = LocalApiRouter::default();

    let rejected = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect repo"}"#,
    );

    assert_eq!(rejected["accepted"], false);
    assert_eq!(rejected["state"], "blocked");
    assert_eq!(rejected["blockedReason"], "setup_not_ready");
    assert_eq!(rejected["nextAction"], "complete_setup");
}

#[test]
fn workbench_route_selects_backend_after_runtime_and_model_readiness() {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );

    let ready = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(ready["route"]["status"], "selected");
    assert_eq!(ready["route"]["backendId"], "backend.ollama");
    assert!(ready["context"].is_object());
}

#[test]
fn workbench_readiness_gate_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/workbench_readiness_gate.rs",
        include_str!("workbench_readiness_gate.rs"),
        130,
    )
    .expect("workbench readiness gate test should stay focused");
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    let _ = route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    let _ = route_json(router, "POST", "/v1/setup/complete", "{}");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
