use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn session_listing_respects_requested_workspace_instead_of_active_workspace() {
    let fixture = TempDir::new().unwrap();
    let first_root = fixture.path().join("first").join("repo");
    let second_root = fixture.path().join("second").join("repo");
    create_repo(&first_root);
    create_repo(&second_root);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let first = open_workspace(&mut router, &first_root);
    router.complete_agent_backend_for_test("First repository answer.");
    let first_session = create_session(&mut router, &first, "First repository task");
    let second = open_workspace(&mut router, &second_root);
    router.complete_agent_backend_for_test("Second repository answer.");
    let second_session = create_session(&mut router, &second, "Second repository task");

    assert_ne!(first["workspaceId"], second["workspaceId"]);
    assert_eq!(first["workspaceId"], "workspace.repo");
    assert!(
        second["workspaceId"]
            .as_str()
            .unwrap()
            .starts_with("workspace.repo.")
    );
    assert_eq!(
        open_workspace(&mut router, &first_root)["workspaceId"],
        first["workspaceId"]
    );
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(state["workspaces"].as_array().unwrap().len(), 2);

    let first_list = get_sessions(&mut router, &first);
    let second_list = get_sessions(&mut router, &second);
    assert_eq!(first_list["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(second_list["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(
        first_list["sessions"][0]["sessionId"],
        first_session["sessionId"]
    );
    assert_eq!(
        second_list["sessions"][0]["sessionId"],
        second_session["sessionId"]
    );
}

#[test]
fn session_workspace_isolation_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_session_workspace_isolation.rs",
        include_str!("local_api_session_workspace_isolation.rs"),
        130,
    )
    .unwrap();
}

fn create_session(router: &mut LocalApiRouter, workspace: &Value, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(workspace["workspaceId"].as_str().unwrap()).unwrap(),
            serde_json::to_string(prompt).unwrap(),
        ),
    )
}

fn get_sessions(router: &mut LocalApiRouter, workspace: &Value) -> Value {
    route_json(
        router,
        "GET",
        &format!(
            "/v1/sessions?workspace_id={}",
            workspace["workspaceId"].as_str().unwrap()
        ),
        "",
    )
}

fn open_workspace(router: &mut LocalApiRouter, root: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&root),
    )
}

fn create_repo(root: &std::path::Path) {
    std::fs::create_dir_all(root).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
