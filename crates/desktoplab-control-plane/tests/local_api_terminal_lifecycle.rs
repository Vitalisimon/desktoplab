use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn terminal_lifecycle_records_pending_approved_and_denied_events() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);

    let pending = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        r#"{"command":"printf pending","cwd":"","approvalRequired":true}"#,
    );
    assert_eq!(pending["state"], "approval_required");
    assert_eq!(pending["approval"]["state"], "pending");

    let approved_id = create_resolved_terminal_approval(&mut router, "approve", "printf approved");
    let completed = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"command":"printf approved","cwd":"","approvalRequired":true,"approvalId":"{approved_id}"}}"#
        ),
    );
    assert_eq!(completed["state"], "completed");
    assert_eq!(completed["events"][0]["stdout"], "approved");

    let denied_id = create_resolved_terminal_approval(&mut router, "deny", "printf denied");
    let denied = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"command":"printf denied","cwd":"","approvalRequired":true,"approvalId":"{denied_id}"}}"#
        ),
    );
    assert_eq!(denied["state"], "denied");
    assert!(denied.get("events").is_none());

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("terminal.approval_required"))
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("terminal.completed"))
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("terminal.denied"))
    );
}

#[test]
fn user_terminal_command_executes_without_approval_by_default() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        r#"{"command":"printf user-terminal-ok","cwd":"","userInitiated":true}"#,
    );

    assert_eq!(response["state"], "completed");
    assert_eq!(response["events"][0]["stdout"], "user-terminal-ok");
    assert_eq!(
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
#[cfg(unix)]
fn user_terminal_command_blocks_symlink_cwd_escape() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace_root = fixture.path().join("workspace");
    let outside_root = fixture.path().join("outside");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    std::fs::create_dir_all(&outside_root).expect("outside should exist");
    std::os::unix::fs::symlink(&outside_root, workspace_root.join("linked-outside"))
        .expect("symlink should be created");
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        r#"{"command":"pwd","cwd":"linked-outside","userInitiated":true}"#,
    );

    assert_eq!(response["state"], "blocked");
    assert_eq!(response["reason"], "path_escape");
}

#[test]
fn local_api_terminal_lifecycle_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_terminal_lifecycle.rs",
        include_str!("local_api_terminal_lifecycle.rs"),
        210,
    )
    .expect("terminal lifecycle test should stay focused");
}

fn create_resolved_terminal_approval(
    router: &mut LocalApiRouter,
    resolution: &str,
    command: &str,
) -> String {
    let created = route_json(
        router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.local","action":"terminal.command","operationId":"workspace.workspace:terminal.local","payload":{{"command":"{command}","cwd":""}}}}"#
        ),
    );
    let approval_id = created["approvalId"].as_str().unwrap().to_string();
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        &format!(r#"{{"resolution":"{resolution}"}}"#),
    );
    approval_id
}

fn open_workspace_after_setup(router: &mut LocalApiRouter, workspace_root: &std::path::Path) {
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(workspace_root)
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
    mark_setup_ready(router);
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
