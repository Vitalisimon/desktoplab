use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn git_commit_requires_payload_bound_reviewed_file_set() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n\nChanged.\n").unwrap();
    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"].as_str().unwrap();

    let approval_id = create_approved_commit_approval(
        &mut router,
        "session.1",
        "docs: update demo",
        change_fingerprint,
        Some(&[]),
    );
    let rejected = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"docs: update demo","changeFingerprint":"{change_fingerprint}","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(rejected["status"], "blocked");
    assert_eq!(rejected["reason"], "missing_reviewed_file_set");
}

#[test]
fn git_commit_rechecks_exact_file_set_after_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n\nChanged.\n").unwrap();
    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"].as_str().unwrap();
    let approval_id = create_approved_commit_approval(
        &mut router,
        "session.1",
        "docs: update demo",
        change_fingerprint,
        Some(&["README.md"]),
    );
    std::fs::write(workspace_root.join("EXTRA.md"), "new\n").unwrap();

    let rejected = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"docs: update demo","changeFingerprint":"{change_fingerprint}","changedFiles":["README.md"],"approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(rejected["status"], "blocked");
    assert_eq!(rejected["reason"], "working_tree_changed_after_approval");
}

#[test]
fn git_commit_with_payload_bound_message_and_file_set_commits_locally_only() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n\nChanged.\n").unwrap();
    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"].as_str().unwrap();
    let approval_id = create_approved_commit_approval(
        &mut router,
        "session.1",
        "docs: update demo",
        change_fingerprint,
        Some(&["README.md"]),
    );

    let committed = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"docs: update demo","changeFingerprint":"{change_fingerprint}","changedFiles":["README.md"],"approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(committed["status"], "committed");
    assert!(!committed.as_object().unwrap().contains_key("pushed"));
    let log = run_git_output(&workspace_root, &["log", "-1", "--pretty=%B"]);
    assert!(log.contains("docs: update demo"));
    assert!(log.contains("DesktopLab-Session: session.1"));
}

#[test]
fn agent_commit_approval_blocks_if_staged_content_changes_before_consumption() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n\nChanged.\n").unwrap();
    run_git(&workspace_root, &["add", "README.md"]);
    router.complete_agent_backend_for_test("Propongo commit.");
    let workspace_id = current_workspace_id(&mut router);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &serde_json::json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"proponi commit",
            "plannedTool":"git.commit",
            "message":"docs: update demo"
        })
        .to_string(),
    );
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
    std::fs::write(
        workspace_root.join("README.md"),
        "# Demo\n\nChanged after approval.\n",
    )
    .unwrap();
    run_git(&workspace_root, &["add", "README.md"]);
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let failed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &serde_json::json!({
            "executionBackendId":"backend.ollama",
            "prompt":"Continue approved commit",
            "approvalId":approval_id
        })
        .to_string(),
    );

    assert_eq!(failed["state"], "failed");
    assert_timeline_contains(&failed, "working_tree_changed_after_approval");
}

fn current_workspace_id(router: &mut LocalApiRouter) -> Value {
    route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone()
}

#[test]
fn local_api_agent_commit_approval_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_commit_approval.rs",
        include_str!("local_api_agent_commit_approval.rs"),
        260,
    )
    .expect("agent commit approval tests should stay focused");
}

fn create_approved_commit_approval(
    router: &mut LocalApiRouter,
    session_id: &str,
    message: &str,
    change_fingerprint: &str,
    changed_files: Option<&[&str]>,
) -> String {
    let changed_files_json = changed_files
        .map(|files| {
            format!(
                r#","changedFiles":[{}]"#,
                files
                    .iter()
                    .map(|file| serde_json::to_string(file).unwrap())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .unwrap_or_default();
    let approval_id = route_json(
        router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"{session_id}","action":"git.commit","operationId":"git.commit","payload":{{"sessionId":"{session_id}","message":{},"changeFingerprint":"{change_fingerprint}"{changed_files_json}}}}}"#,
            serde_json::to_string(message).unwrap()
        ),
    )["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    approval_id
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace_root, &["add", "."]);
    run_git(&workspace_root, &["commit", "-m", "initial"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    (fixture, workspace_root, router)
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| event["message"]
            .as_str()
            .is_some_and(|message| message.contains(expected))),
        "missing {expected}: {session}"
    );
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.enable_test_controls_for_dev_server();
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
    assert!(output.status.success(), "{:?}", args);
}

fn run_git_output(root: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "{:?}", args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
