use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn failed_terminal_command_preserves_evidence_and_returns_control_to_the_agent() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let command = failing_command();
    router.complete_agent_backend_for_test(
        serde_json::json!({
            "name": "desktoplab.run_terminal",
            "arguments": { "command": command }
        })
        .to_string(),
    );
    let blocked = create_session(
        &mut router,
        "run the diagnostic command and report its result",
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.complete","arguments":{"message":"The diagnostic command exited with code 7 and reported terminal-failure-proof."}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let session = &completed["session"];

    assert_eq!(session["state"], "completed", "{session}");
    assert_timeline_contains(session, "state=failed");
    assert_timeline_contains(session, "command_exit_nonzero:7");
    assert_timeline_contains(session, "status=exited:7");
    assert_timeline_contains(session, "terminal-failure-proof");
    assert_timeline_contains(session, "exited with code 7");
}

#[test]
fn terminal_failure_recovery_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_terminal_failure_recovery.rs",
        include_str!("local_api_agent_terminal_failure_recovery.rs"),
        145,
    )
    .expect("terminal failure recovery tests should stay focused");
}

#[cfg(not(windows))]
fn failing_command() -> &'static str {
    "printf terminal-failure-proof >&2; exit 7"
}

#[cfg(windows)]
fn failing_command() -> &'static str {
    "Write-Error terminal-failure-proof; exit 7"
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    (fixture, workspace_root, router)
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    assert!(
        session["timeline"].as_array().unwrap().iter().any(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
        }),
        "missing {expected}: {session}"
    );
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
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    serde_json::from_str(response.body()).expect("response should be JSON")
}
