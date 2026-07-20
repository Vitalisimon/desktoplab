use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn approvals_are_created_listed_and_resolved_by_local_api() {
    let mut router = LocalApiRouter::default();

    let created = route_json(
        &mut router,
        "POST",
        "/v1/approvals",
        r#"{"sessionId":"session.1","action":"terminal.command","operationId":"workspace.workspace:terminal.local","payload":{"command":"printf approved-ok","cwd":""}}"#,
    );
    let approval_id = created["approvalId"]
        .as_str()
        .expect("approval id should be returned")
        .to_string();
    assert_eq!(created["operationId"], "workspace.workspace:terminal.local");

    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"][0]["approvalId"], approval_id);
    assert_eq!(listed["approvals"][0]["state"], "pending");

    let resolved = route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    assert_eq!(resolved["approvalId"], approval_id);
    assert_eq!(resolved["state"], "approved");
    assert_eq!(
        resolved["operationId"],
        "workspace.workspace:terminal.local"
    );
}

#[test]
fn terminal_command_rejects_request_body_self_approval_without_matching_record() {
    let (_fixture, mut router) = router_with_workspace();

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        r#"{"command":"printf should-not-run","cwd":"","approval":"approved","approvalRequired":true}"#,
    );

    assert_eq!(response["state"], "approval_required");
    assert_eq!(response["approval"]["state"], "pending");
    assert!(
        response.get("events").is_none(),
        "self-approved terminal request must not execute: {response}"
    );
}

#[test]
fn approved_terminal_command_requires_resolved_approval_id() {
    let (_fixture, mut router) = router_with_workspace();

    let created = route_json(
        &mut router,
        "POST",
        "/v1/approvals",
        r#"{"sessionId":"session.1","action":"terminal.command","operationId":"workspace.workspace:terminal.local","payload":{"command":"printf approved-ok","cwd":""}}"#,
    );
    let approval_id = created["approvalId"].as_str().unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"sessionId":"session.1","command":"printf approved-ok","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(response["state"], "completed");
    assert_eq!(response["events"][0]["stdout"], "approved-ok");
}

#[test]
fn git_routes_reject_request_body_self_approval_without_matching_record() {
    let (_fixture, mut router) = router_with_git_workspace();

    let rejected_commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        r#"{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"agent change","approval":"approved"}"#,
    );
    assert_eq!(rejected_commit["status"], "blocked");
    assert_eq!(rejected_commit["reason"], "approval_required");

    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"]
        .as_str()
        .expect("git operations should expose change fingerprint");
    let approval_id = route_json(
        &mut router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.1","action":"git.commit","operationId":"git.commit","payload":{{"sessionId":"session.1","message":"agent change","changeFingerprint":"{change_fingerprint}","changedFiles":["README.md"]}}}}"#
        ),
    )["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let approved_commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"agent change","changeFingerprint":"{change_fingerprint}","changedFiles":["README.md"],"approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(approved_commit["status"], "committed");
}

#[test]
fn approval_ids_cannot_be_replayed_across_privileged_actions() {
    let mut router = LocalApiRouter::default();
    let terminal_approval = create_resolved_approval(
        &mut router,
        "terminal.command",
        "workspace.workspace:terminal.local",
    );

    let replayed_commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(r#"{{"message":"wrong action","approvalId":"{terminal_approval}"}}"#),
    );
    assert_eq!(replayed_commit["status"], "blocked");
    assert_eq!(replayed_commit["reason"], "approval_required");

    let replayed_plugin = route_json(
        &mut router,
        "POST",
        "/v1/plugins/plugin.acp/trust",
        &format!(r#"{{"approvalId":"{terminal_approval}"}}"#),
    );
    assert_eq!(replayed_plugin["status"], "approval_required");
    assert_eq!(replayed_plugin["reason"], "approval_record_required");
}

#[test]
fn plugin_trust_approval_cannot_execute_terminal_command() {
    let (_fixture, mut router) = router_with_workspace();
    let plugin_approval = create_resolved_approval(&mut router, "plugin.trust", "plugin.acp");

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"command":"printf replayed","cwd":"","approvalRequired":true,"approvalId":"{plugin_approval}"}}"#
        ),
    );

    assert_eq!(response["state"], "approval_required");
    assert!(response.get("events").is_none());
}

#[test]
fn local_api_approval_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_approvals.rs",
        include_str!("local_api_approvals.rs"),
        260,
    )
    .expect("local api approval test should stay focused");
}

fn create_resolved_approval(
    router: &mut LocalApiRouter,
    action: &str,
    operation_id: &str,
) -> String {
    let created = route_json(
        router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.1","action":"{action}","operationId":"{operation_id}"}}"#
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
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    open_workspace_after_setup(&mut router, &workspace_root);
    (fixture, router)
}

fn router_with_git_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
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
