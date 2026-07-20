use desktoplab_control_plane::LocalApiRouter;
use tempfile::TempDir;

#[test]
fn archived_active_session_is_hidden_from_workspace_and_session_list() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut router = LocalApiRouter::with_storage_path(&db_path).expect("router should open");
    mark_setup_ready(&mut router);
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );
    router.complete_agent_backend_for_test("Archived answer.");
    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Archived prompt"}"#,
    );
    let session_id = created["sessionId"].as_str().unwrap();

    route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/archive"),
        "{}",
    );
    let listed = route_json(
        &mut router,
        "GET",
        "/v1/sessions?workspace_id=workspace.desktoplab",
        "",
    );
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert!(listed["sessions"].as_array().unwrap().is_empty());
    assert!(workspace["session"].is_null());

    drop(router);
    let mut restarted = LocalApiRouter::with_storage_path(&db_path).expect("router should restart");
    let restarted_list = route_json(
        &mut restarted,
        "GET",
        "/v1/sessions?workspace_id=workspace.desktoplab",
        "",
    );
    let restarted_workspace = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert!(restarted_list["sessions"].as_array().unwrap().is_empty());
    assert!(restarted_workspace["session"].is_null());
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

fn route_json(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> serde_json::Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
