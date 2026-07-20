use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn completed_write_child_exposes_committed_changes_for_parent_review() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());
    let child = spawn_write_child(&mut router, &parent_id);
    let child_id = child["subagentId"].as_str().unwrap();
    let worktree = std::path::Path::new(child["worktree"].as_str().unwrap());

    std::fs::write(worktree.join("child.md"), "# Reviewed child change\n").unwrap();
    run_git(worktree, &["add", "child.md"]);
    run_git(worktree, &["commit", "-m", "add reviewed child change"]);
    cancel_child(&mut router, child_id);

    let status = get_child(&mut router, child_id);
    let review = &status["changeReview"];
    assert_eq!(review["status"], "reviewable", "{status}");
    assert_eq!(review["workingTreeClean"], true, "{status}");
    assert_eq!(review["readyToIntegrate"], true, "{status}");
    assert_eq!(review["changedFiles"], serde_json::json!(["child.md"]));
    assert_eq!(review["commits"].as_array().unwrap().len(), 1);
    assert!(
        review["diffPreview"]
            .as_str()
            .unwrap()
            .contains("Reviewed child change")
    );
    assert_ne!(review["baseCommit"], review["headCommit"]);
}

#[test]
fn uncommitted_write_child_is_not_claimed_as_ready_to_integrate() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());
    let child = spawn_write_child(&mut router, &parent_id);
    let child_id = child["subagentId"].as_str().unwrap();
    let worktree = std::path::Path::new(child["worktree"].as_str().unwrap());

    std::fs::write(worktree.join("unfinished.md"), "not committed\n").unwrap();
    cancel_child(&mut router, child_id);

    let status = get_child(&mut router, child_id);
    let review = &status["changeReview"];
    assert_eq!(review["status"], "blocked", "{status}");
    assert_eq!(review["workingTreeClean"], false, "{status}");
    assert_eq!(review["readyToIntegrate"], false, "{status}");
    assert_eq!(review["reason"], "uncommitted_changes", "{status}");
    assert_eq!(review["changedFiles"], serde_json::json!(["unfinished.md"]));
}

fn spawn_write_child(router: &mut LocalApiRouter, parent_id: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/agent/subagents",
        &format!(
            r#"{{"parentSessionId":"{parent_id}","prompt":"prepare a reviewed change","intent":"write_capable"}}"#
        ),
    )
}

fn cancel_child(router: &mut LocalApiRouter, child_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/agent/subagents/{child_id}/cancel"),
        "{}",
    );
}

fn get_child(router: &mut LocalApiRouter, child_id: &str) -> Value {
    route_json(
        router,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    )
}

fn create_parent(router: &mut LocalApiRouter) -> String {
    router.complete_agent_backend_for_test("Parent ready.");
    route_json(
        router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"prepare parent"}"#,
    )["sessionId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (fixture, router)
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
        .args(["-c", "user.name=DesktopLab", "-c", "user.email=x@y.z"])
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

#[test]
fn subagent_change_review_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_subagent_change_review.rs",
        include_str!("local_api_subagent_change_review.rs"),
        210,
    )
    .expect("subagent change review test should stay focused");
}
