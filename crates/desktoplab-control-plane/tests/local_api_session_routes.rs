use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn session_routes_survive_router_restart_from_sqlite() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");

    let mut first_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should open storage");
    mark_setup_ready(&mut first_router);
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    post(
        &mut first_router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );
    first_router.complete_agent_backend_for_test("Stored backend response.");
    let created = route_json(
        &mut first_router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect the repository"}"#,
    );
    assert_eq!(created["sessionId"], "session.1");
    assert_eq!(created["plan"], "Inspect the repository");
    assert_eq!(created["state"], "completed");

    let mut restarted_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should reopen storage");
    let listed = route_json(
        &mut restarted_router,
        "GET",
        "/v1/sessions?workspace_id=workspace.desktoplab",
        "",
    );

    assert_eq!(listed["sessions"][0]["sessionId"], "session.1");
    assert_eq!(listed["sessions"][0]["owner"], "desktoplab");
    assert_eq!(listed["sessions"][0]["plan"], "Inspect the repository");
    assert_eq!(listed["sessions"][0]["state"], "completed");
}

#[test]
fn local_api_session_routes_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_session_routes.rs",
        include_str!("local_api_session_routes.rs"),
        150,
    )
    .expect("session route test should stay focused");
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
