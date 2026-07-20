use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn terminal_approval_is_bound_to_exact_command_payload_and_is_one_shot() {
    let (_fixture, mut router) = router_with_workspace();
    let approval_id = create_payload_approval(
        &mut router,
        "terminal.command",
        "workspace.workspace:terminal.local",
        r#"{"command":"printf approved","cwd":""}"#,
    );

    let changed_command = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"sessionId":"session.1","command":"printf changed","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(changed_command["state"], "approval_required");
    assert!(changed_command.get("events").is_none());

    let approved_command = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"sessionId":"session.1","command":"printf approved","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(approved_command["state"], "completed");
    assert_eq!(approved_command["events"][0]["stdout"], "approved");

    let replayed_command = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"sessionId":"session.1","command":"printf approved","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(replayed_command["state"], "approval_required");
}

#[test]
fn git_commit_approval_is_bound_to_exact_message_payload_and_is_one_shot() {
    let (_fixture, mut router) = router_with_git_workspace();
    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"]
        .as_str()
        .expect("commit preview should include exact change fingerprint");
    let changed_files = serde_json::to_string(
        operations["changedFiles"]
            .as_array()
            .expect("commit preview should include changed files"),
    )
    .expect("changed files should serialize");
    let approval_id = create_payload_approval(
        &mut router,
        "git.commit",
        "git.commit",
        &format!(
            r#"{{"sessionId":"session.1","message":"approved message","changeFingerprint":"{change_fingerprint}","changedFiles":{changed_files}}}"#
        ),
    );

    let changed_message = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"sessionId":"session.1","message":"changed message","approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(changed_message["status"], "blocked");
    assert_eq!(changed_message["reason"], "approval_required");

    let approved_commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"sessionId":"session.1","message":"approved message","changeFingerprint":"{change_fingerprint}","changedFiles":{changed_files},"approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(approved_commit["status"], "committed");

    std::fs::write(
        _fixture.path().join("workspace").join("SECOND.md"),
        "second change\n",
    )
    .expect("second fixture file should write");
    run_git(&_fixture.path().join("workspace"), &["add", "SECOND.md"]);
    let replayed_commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"sessionId":"session.1","message":"approved message","approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(replayed_commit["status"], "blocked");
    assert_eq!(replayed_commit["reason"], "approval_required");
}

#[test]
fn local_api_payload_approval_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_payload_approvals.rs",
        include_str!("local_api_payload_approvals.rs"),
        260,
    )
    .expect("payload approval test should stay focused");
}

fn create_payload_approval(
    router: &mut LocalApiRouter,
    action: &str,
    operation_id: &str,
    payload: &str,
) -> String {
    let created = route_json(
        router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.1","action":"{action}","operationId":"{operation_id}","payload":{payload}}}"#
        ),
    );
    let approval_id = created["approvalId"].as_str().unwrap().to_string();
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    approval_id
}

fn router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init"]);
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);
    (fixture, router)
}

fn router_with_git_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init"]);
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.local"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace_root.join("README.md"), "DesktopLab\n")
        .expect("fixture file should write");
    run_git(&workspace_root, &["add", "README.md"]);
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);
    (fixture, router)
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

fn open_workspace_after_setup(router: &mut LocalApiRouter, workspace_root: &std::path::Path) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
    post(
        router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
