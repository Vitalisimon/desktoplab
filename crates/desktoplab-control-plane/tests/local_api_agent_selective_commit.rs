use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn provider_commit_can_select_reviewed_files_without_absorbing_other_changes() {
    let (_fixture, workspace, mut router) = router_with_workspace();
    std::fs::write(workspace.join("README.md"), "# Selected\n").unwrap();
    std::fs::write(workspace.join("EXTRA.md"), "leave uncommitted\n").unwrap();
    run_git(&workspace, &["add", "EXTRA.md"]);
    router.complete_agent_backend_for_test(
        r#"{"tool":"desktoplab.commit_changes","arguments":{"message":"docs: selected change","paths":["README.md"]}}"#,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"commit only README.md"}"#,
    );
    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_eq!(
        blocked["pendingApprovals"][0]["details"]["changedFiles"],
        serde_json::json!(["README.md"]),
        "{blocked}"
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        git(&workspace, &["show", "--format=", "--name-only", "HEAD"]),
        "README.md"
    );
    assert!(git(&workspace, &["status", "--porcelain"]).contains("A  EXTRA.md"));
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    run_git(
        &workspace,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace.join("README.md"), "# Initial\n").unwrap();
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    router.set_host_memory_gb_for_test(32);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (fixture, workspace, router)
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

#[test]
fn selective_commit_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_selective_commit.rs",
        include_str!("local_api_agent_selective_commit.rs"),
        145,
    )
    .expect("selective commit test should stay focused");
}
