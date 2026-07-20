use desktoplab_control_plane::LocalApiRouter;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn session_continue_appends_to_existing_thread_without_creating_a_second_session() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let repo = fixture.path().join("desktoplab");
    create_repo(&repo);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    open_workspace(&mut router, &repo);
    router.complete_agent_backend_for_test("First answer.");
    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"First prompt"}"#,
    );
    let session_id = created["sessionId"].as_str().unwrap();

    router.complete_agent_backend_for_test("Second answer.");
    let continued = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","prompt":"Second prompt"}"#,
    );
    let sessions = route_json(
        &mut router,
        "GET",
        "/v1/sessions?workspace_id=workspace.desktoplab",
        "",
    );

    assert_eq!(continued["sessionId"], session_id);
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 1);
    assert!(continued["timeline"].to_string().contains("First prompt"));
    assert!(continued["timeline"].to_string().contains("First answer."));
    assert!(continued["timeline"].to_string().contains("Second prompt"));
    assert!(continued["timeline"].to_string().contains("Second answer."));
    let timeline = continued["timeline"].as_array().unwrap();
    let messages = timeline
        .iter()
        .map(|event| event["message"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(message_index(&messages, "First prompt") < message_index(&messages, "First answer."));
    assert!(message_index(&messages, "First answer.") < message_index(&messages, "Second prompt"));
    assert!(message_index(&messages, "Second prompt") < message_index(&messages, "Second answer."));
    assert_ne!(
        continued["timeline"][0]["createdAt"], "2026-06-26T08:00:00Z",
        "timeline timestamps must not be hardcoded demo time"
    );
}

#[test]
fn app_state_keeps_opened_projects_until_the_user_archives_them() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let first_repo = fixture.path().join("First");
    let second_repo = fixture.path().join("Second");
    create_repo(&first_repo);
    create_repo(&second_repo);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let first = open_workspace(&mut router, &first_repo);
    let second = open_workspace(&mut router, &second_repo);
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(
        state["currentWorkspace"]["workspaceId"],
        second["workspaceId"]
    );
    assert_eq!(state["workspaces"].as_array().unwrap().len(), 2);
    assert!(state["workspaces"].to_string().contains("First"));
    assert!(state["workspaces"].to_string().contains("Second"));

    route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/workspaces/{}/archive",
            first["workspaceId"].as_str().unwrap()
        ),
        "{}",
    );
    let archived_state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(archived_state["workspaces"].as_array().unwrap().len(), 1);
    assert!(!archived_state["workspaces"].to_string().contains("First"));
    assert!(archived_state["workspaces"].to_string().contains("Second"));
}

#[test]
fn agent_workspace_prefers_backend_active_thread_over_latest_thread() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let repo = fixture.path().join("desktoplab");
    create_repo(&repo);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    open_workspace(&mut router, &repo);
    router.complete_agent_backend_for_test("First answer.");
    let first = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"First prompt"}"#,
    );
    router.complete_agent_backend_for_test("Second answer.");
    let second = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Second prompt"}"#,
    );
    assert_ne!(first["sessionId"], second["sessionId"]);

    router.complete_agent_backend_for_test("First followup.");
    route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            first["sessionId"].as_str().unwrap()
        ),
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","prompt":"First followup"}"#,
    );
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace["session"]["sessionId"], first["sessionId"]);
    assert!(
        workspace["session"]["timeline"]
            .to_string()
            .contains("First followup")
    );
}

#[test]
fn thread_project_truth_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_thread_project_truth.rs",
        include_str!("local_api_thread_project_truth.rs"),
        210,
    )
    .expect("thread/project truth tests should stay focused");
}

fn open_workspace(router: &mut LocalApiRouter, path: &std::path::Path) -> serde_json::Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    )
}

fn message_index(messages: &[&str], expected: &str) -> usize {
    messages
        .iter()
        .position(|message| *message == expected)
        .unwrap_or_else(|| panic!("{expected} should be in timeline: {messages:?}"))
}

fn create_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("repo dir should be writable");
    std::process::Command::new("git")
        .arg("init")
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
