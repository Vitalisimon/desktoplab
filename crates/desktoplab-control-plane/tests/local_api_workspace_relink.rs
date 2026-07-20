use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn relink_missing_workspace_keeps_thread_history_and_reenables_input() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let original = fixture.path().join("Original");
    let moved = fixture.path().join("Moved");
    create_repo(&original);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let workspace = open_workspace(&mut router, &original);
    router.complete_agent_backend_for_test("Existing answer remains readable.");
    let created = create_session(&mut router, workspace_id(&workspace), "Existing prompt");
    std::fs::remove_dir_all(&original).expect("original repo should be removable");
    create_repo(&moved);

    let missing = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(missing["currentWorkspace"]["readOnly"], true);
    let relinked = route_json(
        &mut router,
        "POST",
        &format!("/v1/workspaces/{}/relink", workspace_id(&workspace)),
        &xtask::test_http::workspace_open_body(&moved),
    );
    router.complete_agent_backend_for_test("Input works after relink.");
    let continued = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            created["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{}","executionBackendId":"backend.ollama","prompt":"continue"}}"#,
            workspace_id(&workspace)
        ),
    );

    assert_eq!(relinked["workspaceId"], workspace["workspaceId"]);
    assert_eq!(relinked["readOnly"], false);
    assert_eq!(continued["state"], "completed");
    let sessions = route_json(
        &mut router,
        "GET",
        &format!("/v1/sessions?workspace_id={}", workspace_id(&workspace)),
        "",
    );
    assert_eq!(sessions["sessions"][0]["sessionId"], created["sessionId"]);
}

#[test]
fn workspace_relink_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_workspace_relink.rs",
        include_str!("local_api_workspace_relink.rs"),
        120,
    )
    .expect("workspace relink test should stay focused");
}

fn create_session(router: &mut LocalApiRouter, workspace_id: &str, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"{prompt}"}}"#
        ),
    )
}

fn open_workspace(router: &mut LocalApiRouter, path: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    )
}

fn workspace_id(workspace: &Value) -> &str {
    workspace["workspaceId"].as_str().unwrap()
}

fn create_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("repo dir should be writable");
    run_git(path, &["init", "-b", "main"]);
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

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "git {:?} failed", args);
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
