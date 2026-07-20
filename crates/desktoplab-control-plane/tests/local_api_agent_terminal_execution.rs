use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn agent_terminal_command_requires_approval_and_records_output_observation() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Eseguo il comando dopo approvazione.","tool":"desktoplab.run_terminal","arguments":{"command":"printf agent-terminal-ok","reason":"terminal evidence test"}}"#,
    );

    let blocked = create_session(&mut router, "esegui printf agent-terminal-ok");

    assert_eq!(blocked["state"], "blocked");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    let approval = &listed["approvals"][0];
    assert_eq!(approval["action"], "terminal.command");
    assert_eq!(approval["operationId"], "terminal:printf agent-terminal-ok");
    assert_eq!(approval["title"], "Run printf agent-terminal-ok");
    assert_eq!(
        approval["message"],
        "DesktopLab wants to run `printf agent-terminal-ok` in the workspace terminal."
    );
    let approval_id = approval["approvalId"].as_str().unwrap().to_string();

    resolve_approval(&mut router, &approval_id);
    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &continuation_body(&approval_id),
    );

    assert_eq!(completed["state"], "completed");
    assert_tool_decision(&completed, "terminal:printf agent-terminal-ok");
    assert_tool_output_contains(&completed, "agent-terminal-ok");
    assert_tool_output_contains(&completed, "duration_ms=");
    assert_tool_output_contains(&completed, "cwd=");
}

#[test]
fn denied_agent_terminal_approval_does_not_run_command() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Comando in attesa di approvazione.","tool":"desktoplab.run_terminal","arguments":{"command":"printf should-not-run > terminal-marker.txt","reason":"denied terminal test"}}"#,
    );

    let blocked = create_session(
        &mut router,
        "esegui printf should-not-run > terminal-marker.txt",
    );
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();

    deny_approval(&mut router, &approval_id);
    let denied = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &continuation_body(&approval_id),
    );

    assert_eq!(denied["state"], "blocked");
    assert!(!workspace_root.join("terminal-marker.txt").exists());
}

#[test]
fn agent_terminal_execution_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_terminal_execution.rs",
        include_str!("local_api_agent_terminal_execution.rs"),
        185,
    )
    .expect("agent terminal execution test should stay focused");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            workspace_id,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn continuation_body(approval_id: &str) -> String {
    serde_json::json!({
        "executionBackendId":"backend.ollama",
        "prompt":"continue",
        "approvalId":approval_id
    })
    .to_string()
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn deny_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"deny"}"#,
    );
}

fn assert_tool_decision(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "tool_decision"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
}

fn assert_tool_output_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "tool"
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
