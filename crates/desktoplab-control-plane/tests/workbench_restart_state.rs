use desktoplab_control_plane::LocalApiRouter;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workbench_workspace_and_completed_prompt_survive_router_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("repo");
    create_repo(&workspace_root);

    let mut first_router = LocalApiRouter::with_storage_path(&db_path).expect("router should open");
    mark_setup_ready(&mut first_router);
    first_router.complete_agent_backend_for_test("Restart-safe backend response.");
    let workspace = route_json(
        &mut first_router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    let workspace_id = workspace["workspaceId"]
        .as_str()
        .expect("workspace id should exist");

    let created = route_json(
        &mut first_router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Recover this prompt after restart"}}"#
        ),
    );
    assert_eq!(created["state"], "completed");
    assert_eq!(
        created["timeline"][1]["message"],
        "Restart-safe backend response."
    );
    assert_eq!(created["trace"]["schemaVersion"], 1);
    assert_eq!(created["trace"]["sessionId"], created["sessionId"]);
    let created_trace = serde_json::to_string(&created["trace"]).unwrap();
    assert!(!created_trace.contains("Recover this prompt after restart"));
    assert!(created_trace.contains("user_prompt_recorded"));

    let mut restarted = LocalApiRouter::with_storage_path(&db_path).expect("router should restart");
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");
    assert_eq!(state["currentWorkspace"]["workspaceId"], workspace_id);
    assert_eq!(state["routeInput"]["activeSessionCount"], 1);

    let sessions = route_json(&mut restarted, "GET", "/v1/sessions", "");
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(sessions["sessions"][0]["state"], "completed");
    assert!(
        sessions["sessions"][0]["timeline"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Recover this prompt after restart")
    );

    let workbench = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert_eq!(workbench["session"]["state"], "completed");
    assert_eq!(
        workbench["session"]["timeline"][1]["message"],
        "Restart-safe backend response."
    );
    assert_eq!(workbench["session"]["trace"], created["trace"]);
}

#[test]
fn selected_historical_thread_survives_router_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("repo");
    create_repo(&workspace_root);

    let mut first_router = LocalApiRouter::with_storage_path(&db_path).expect("router should open");
    mark_setup_ready(&mut first_router);
    let workspace = route_json(
        &mut first_router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    let workspace_id = workspace["workspaceId"].as_str().unwrap();
    first_router.complete_agent_backend_for_test("First answer.");
    let first = create_session(&mut first_router, workspace_id, "First prompt");
    first_router.complete_agent_backend_for_test("Second answer.");
    let second = create_session(&mut first_router, workspace_id, "Second prompt");
    first_router.complete_agent_backend_for_test("Continue first.");
    let continued_first = route_json(
        &mut first_router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            first["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"continue first"}}"#
        ),
    );

    assert_eq!(continued_first["sessionId"], first["sessionId"]);
    assert_ne!(first["sessionId"], second["sessionId"]);

    let mut restarted = LocalApiRouter::with_storage_path(&db_path).expect("router should restart");
    let workbench = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");

    assert_eq!(workbench["session"]["sessionId"], first["sessionId"]);
    assert!(
        workbench["session"]["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["message"].as_str() == Some("continue first")),
        "{workbench}"
    );
}

#[test]
fn workbench_restart_state_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/workbench_restart_state.rs",
        include_str!("workbench_restart_state.rs"),
        210,
    )
    .expect("workbench restart state test should stay focused");
}

fn create_session(
    router: &mut LocalApiRouter,
    workspace_id: &str,
    prompt: &str,
) -> serde_json::Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"{prompt}"}}"#
        ),
    )
}

fn create_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("workspace should exist");
    std::fs::write(path.join("AGENTS.md"), "# DesktopLab restart proof\n")
        .expect("agent instructions should be writable");
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .expect("git init should run");
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
