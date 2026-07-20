use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn agent_reviews_git_diff_before_commit_proposal() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n\nChanged.\n")
        .expect("fixture should change");
    router.complete_agent_backend_for_test("Rivedo il diff prima del commit.");

    let diff_body = session_body(&mut router, "mostra il diff", "git.diff", None);
    let diff = route_json(&mut router, "POST", "/v1/sessions", &diff_body);

    assert_eq!(diff["state"], "completed");
    assert_timeline_contains(&diff, "git.diff");
    assert_timeline_contains(&diff, "diff --git");

    router.complete_agent_backend_for_test("Propongo commit: docs: update demo.");
    let commit_body = session_body(
        &mut router,
        "proponi commit",
        "git.commit",
        Some("docs: update demo"),
    );
    let commit = route_json(&mut router, "POST", "/v1/sessions", &commit_body);

    assert_eq!(commit["state"], "blocked");
    let approval = latest_approval(&mut router);
    assert_eq!(approval["action"], "git.commit");
    assert!(
        approval["payloadHash"]
            .as_str()
            .is_some_and(|hash| !hash.is_empty())
    );
}

fn session_body(
    router: &mut LocalApiRouter,
    prompt: &str,
    planned_tool: &str,
    message: Option<&str>,
) -> String {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    serde_json::json!({
        "workspaceId":workspace_id,
        "executionBackendId":"backend.ollama",
        "initialPrompt":prompt,
        "plannedTool":planned_tool,
        "message":message
    })
    .to_string()
}

#[test]
fn agent_git_flow_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_git_flow.rs",
        include_str!("local_api_agent_git_flow.rs"),
        140,
    )
    .expect("agent git flow test should stay focused");
}

fn latest_approval(router: &mut LocalApiRouter) -> Value {
    let listed = route_json(router, "GET", "/v1/approvals", "");
    listed["approvals"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone()
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
