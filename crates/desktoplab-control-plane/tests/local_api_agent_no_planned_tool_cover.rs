use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn public_session_api_rejects_client_supplied_planned_tool_cover() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);

    let response = router
        .route(
            "POST",
            "/v1/sessions",
            r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"mostra il diff","plannedTool":"git.diff"}"#,
        )
        .expect("session route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    let value: Value = serde_json::from_str(response.body()).expect("response should be json");
    assert_eq!(value["code"], "PLANNED_TOOL_TEST_HARNESS_ONLY");
}

#[test]
fn planned_tool_cover_guard_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_no_planned_tool_cover.rs",
        include_str!("local_api_agent_no_planned_tool_cover.rs"),
        90,
    )
    .expect("planned tool cover guard should stay focused");
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(router, "POST", "/v1/setup/complete", "{}");
}

fn open_test_workspace(router: &mut LocalApiRouter) -> TempDir {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let root = fixture.path().join("desktoplab");
    std::fs::create_dir(&root).expect("workspace should be created");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    fixture
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
