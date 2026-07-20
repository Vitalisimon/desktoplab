use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn write_capable_agent_worktree_route_creates_isolated_worktree_with_merge_policy() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let session_id = create_completed_session(&mut router);

    let route = route_json(
        &mut router,
        "POST",
        "/v1/agent/worktrees",
        &format!(r#"{{"sessionId":"{session_id}","intent":"write_capable"}}"#),
    );

    assert_eq!(route["status"], "ready");
    assert_eq!(
        route["isolationReason"],
        "write_capable_parallel_requires_worktree"
    );
    let worktree_path = route["worktreePath"].as_str().unwrap();
    assert!(std::path::Path::new(worktree_path).is_dir());
    assert_ne!(worktree_path, workspace_root.display().to_string());
    assert_eq!(route["mergePolicy"]["requiresExplicitApproval"], true);
}

#[test]
fn created_agent_worktree_can_be_cleaned_up_by_id() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let session_id = create_completed_session(&mut router);
    let route = route_json(
        &mut router,
        "POST",
        "/v1/agent/worktrees",
        &format!(r#"{{"sessionId":"{session_id}","intent":"write_capable"}}"#),
    );
    let worktree_path = route["worktreePath"].as_str().unwrap().to_string();
    let worktree_id = route["worktreeId"].as_str().unwrap();
    assert!(std::path::Path::new(&worktree_path).exists());

    let cleaned = route_json(
        &mut router,
        "POST",
        &format!("/v1/git/worktrees/{worktree_id}/cleanup"),
        "{}",
    );

    assert_eq!(cleaned["status"], "cleaned");
    assert!(!std::path::Path::new(&worktree_path).exists());
}

#[test]
fn bound_session_executes_approved_file_action_inside_its_worktree() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let session_id = create_completed_session(&mut router);
    let route = route_json(
        &mut router,
        "POST",
        "/v1/agent/worktrees",
        &format!(r#"{{"sessionId":"{session_id}","intent":"write_capable"}}"#),
    );
    let worktree = std::path::Path::new(route["worktreePath"].as_str().unwrap());
    router.complete_agent_backend_for_test(
        r#"{"tool":"desktoplab.write_file","arguments":{"path":"isolated.md","content":"worktree proof\n"}}"#,
    );
    let blocked = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"create the isolated proof"}"#,
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
    route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(
        std::fs::read_to_string(worktree.join("isolated.md")).unwrap(),
        "worktree proof\n"
    );
    assert!(!workspace_root.join("isolated.md").exists());
}

#[test]
fn dirty_managed_worktree_is_not_force_removed() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let session_id = create_completed_session(&mut router);
    let route = route_json(
        &mut router,
        "POST",
        "/v1/agent/worktrees",
        &format!(r#"{{"sessionId":"{session_id}","intent":"write_capable"}}"#),
    );
    let worktree_path = route["worktreePath"].as_str().unwrap();
    std::fs::write(std::path::Path::new(worktree_path).join("dirty.md"), "keep").unwrap();

    let cleanup = route_json(
        &mut router,
        "POST",
        &format!("/v1/git/worktrees/{session_id}/cleanup"),
        "{}",
    );

    assert_eq!(cleanup["status"], "blocked");
    assert!(std::path::Path::new(worktree_path).exists());
}

#[test]
fn agent_worktree_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_worktrees.rs",
        include_str!("local_api_agent_worktrees.rs"),
        245,
    )
    .expect("agent worktree route tests should stay focused");
}

fn create_completed_session(router: &mut LocalApiRouter) -> String {
    router.complete_agent_backend_for_test("Session ready for isolated continuation.");
    route_json(
        router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"prepare isolated session"}"#,
    )["sessionId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").expect("README should write");
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
        .expect("git command should run");
    assert!(
        output.status.success(),
        "{}",
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
