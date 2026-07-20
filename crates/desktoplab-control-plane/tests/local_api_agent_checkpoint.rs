use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn terminal_mutation_requires_persisted_checkpoint_before_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Comando pronto per approvazione.","tool":"desktoplab.run_terminal","arguments":{"command":"printf checkpoint-ok","reason":"checkpoint proof"}}"#,
    );

    let blocked = create_session(&mut router, "esegui printf checkpoint-ok");

    assert_eq!(blocked["state"], "blocked");
    assert_timeline_contains(&blocked, "checkpoint_ready");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    let approval = &listed["approvals"][0];
    assert_eq!(approval["action"], "terminal.command");
    assert!(
        approval["payloadHash"]
            .as_str()
            .is_some_and(|hash| !hash.is_empty())
    );
    assert_eq!(
        git_stdout(
            &workspace_root,
            &[
                "rev-parse",
                "--verify",
                "refs/desktoplab/savepoints/checkpoint.agent.session.1",
            ],
        )
        .trim()
        .len(),
        40
    );
}

#[test]
fn dirty_terminal_mutation_creates_checkpoint_before_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("dirty.txt"), "uncommitted")
        .expect("workspace should become dirty");
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Comando pronto per approvazione.","tool":"desktoplab.run_terminal","arguments":{"command":"printf should-not-run","reason":"checkpoint proof"}}"#,
    );

    let blocked = create_session(&mut router, "esegui printf should-not-run");

    assert_eq!(blocked["state"], "blocked");
    assert_timeline_contains(&blocked, "checkpoint_ready");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"][0]["action"], "terminal.command");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("dirty.txt")).unwrap(),
        "uncommitted"
    );
    assert_eq!(
        git_stdout(
            &workspace_root,
            &[
                "show",
                "refs/desktoplab/savepoints/checkpoint.agent.session.1:dirty.txt",
            ],
        ),
        "uncommitted"
    );
}

#[test]
fn read_only_terminal_call_on_dirty_worktree_remains_approval_gated() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("dirty.txt"), "uncommitted").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.run_terminal","arguments":{"command":"git status && git diff"}}"#,
    );

    let blocked = create_session(&mut router, "show git status and diff without mutating");

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_timeline_contains(&blocked, "checkpoint_ready");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"][0]["action"], "terminal.command");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("dirty.txt")).unwrap(),
        "uncommitted"
    );
}

#[test]
fn continuation_on_dirty_worktree_requests_terminal_approval_after_read() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("dirty.txt"), "uncommitted").unwrap();
    std::fs::write(workspace_root.join("notes.md"), "inspect me\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
        r#"{"name":"desktoplab.run_terminal","arguments":{"command":"npm test"}}"#,
    ]);

    let blocked = create_session(&mut router, "inspect notes and run the tests");

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_eq!(blocked["blockedReason"], "waiting for approval");
    assert_timeline_contains(&blocked, "checkpoint_ready");
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(approvals["approvals"][0]["action"], "terminal.command");
}

#[test]
fn agent_checkpoint_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_checkpoint.rs",
        include_str!("local_api_agent_checkpoint.rs"),
        190,
    )
    .expect("agent checkpoint test should stay focused");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(&workspace_id).unwrap(),
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "tool_decision"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
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
    let _ = git_stdout(root, args);
}

fn git_stdout(root: &std::path::Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).expect("git output should be utf8")
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
