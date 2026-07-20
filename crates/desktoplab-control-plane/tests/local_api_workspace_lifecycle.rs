use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn missing_workspace_root_keeps_history_read_only_and_blocks_new_input() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let repo = fixture.path().join("moved-repo");
    create_repo(&repo);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let workspace = open_workspace(&mut router, &repo);
    router.complete_agent_backend_for_test("Existing answer remains readable.");
    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{}","executionBackendId":"backend.ollama","initialPrompt":"Existing prompt"}}"#,
            workspace["workspaceId"].as_str().unwrap()
        ),
    );
    std::fs::remove_dir_all(&repo).expect("repo should be removable");

    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(state["currentWorkspace"]["stale"], true);
    assert_eq!(state["currentWorkspace"]["readOnly"], true);
    assert_eq!(
        state["currentWorkspace"]["blockedReason"],
        "workspace_root_missing"
    );
    let sessions = route_json(
        &mut router,
        "GET",
        &format!(
            "/v1/sessions?workspace_id={}",
            workspace["workspaceId"].as_str().unwrap()
        ),
        "",
    );
    assert_eq!(sessions["sessions"][0]["sessionId"], created["sessionId"]);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{}","executionBackendId":"backend.ollama","initialPrompt":"Prova a creare il file"}}"#,
            workspace["workspaceId"].as_str().unwrap()
        ),
    );
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "workspace_root_missing");
    assert_eq!(blocked["nextAction"], "relink_workspace");
}

#[test]
fn workspace_lifecycle_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_workspace_lifecycle.rs",
        include_str!("local_api_workspace_lifecycle.rs"),
        140,
    )
    .expect("workspace lifecycle test should stay focused");
}

fn open_workspace(router: &mut LocalApiRouter, path: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    )
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
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
