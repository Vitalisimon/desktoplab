use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn worktree_binding_survives_restart_and_remains_the_execution_root() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("README.md"), "# Proof\n").unwrap();
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    let mut router = LocalApiRouter::with_storage_path(&database).unwrap();
    mark_setup_ready(&mut router);
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    let workspace_id = opened["workspaceId"].as_str().unwrap();
    router.complete_agent_backend_for_test("Session ready.");
    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"prepare"}}"#
        ),
    );
    let session_id = session["sessionId"].as_str().unwrap().to_string();
    let binding = route_json(
        &mut router,
        "POST",
        "/v1/agent/worktrees",
        &format!(r#"{{"sessionId":"{session_id}","intent":"write_capable"}}"#),
    );
    let worktree = binding["worktreePath"].as_str().unwrap().to_string();
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    restarted.complete_agent_backend_for_test(
        r#"{"tool":"desktoplab.write_file","arguments":{"path":"restart-proof.md","content":"isolated after restart\n"}}"#,
    );
    let blocked = route_json(
        &mut restarted,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"write restart proof"}}"#
        ),
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    route_json(
        &mut restarted,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    route_json(
        &mut restarted,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(
        std::fs::read_to_string(std::path::Path::new(&worktree).join("restart-proof.md")).unwrap(),
        "isolated after restart\n"
    );
    assert!(!workspace.join("restart-proof.md").exists());
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
        .args([
            "-c",
            "user.name=DesktopLab",
            "-c",
            "user.email=desktoplab@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

#[test]
fn worktree_persistence_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_worktree_persistence.rs",
        include_str!("local_api_worktree_persistence.rs"),
        165,
    )
    .expect("worktree persistence test should stay focused");
}
