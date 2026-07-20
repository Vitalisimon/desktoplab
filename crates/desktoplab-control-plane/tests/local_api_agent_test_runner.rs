use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn planned_run_tests_uses_approval_and_records_validation_evidence() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Eseguo la validazione richiesta.");

    let blocked = create_session_with(
        &mut router,
        json!({
            "initialPrompt":"valida la modifica",
            "plannedTool":"desktoplab.run_tests",
            "command":"printf validation-ok",
            "reason":"smallest meaningful validation"
        }),
    );

    assert_eq!(blocked["state"], "blocked");
    let approval = &route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0];
    assert_eq!(approval["action"], "test.run");
    assert_eq!(approval["operationId"], "test.run:printf validation-ok");
    let approval_id = approval["approvalId"].as_str().unwrap().to_string();
    approve(&mut router, &approval_id);

    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &json!({"executionBackendId":"backend.ollama","prompt":"continue","approvalId":approval_id}).to_string(),
    );

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "test.run:printf validation-ok");
    assert_timeline_contains(&completed, "validation-ok");
    assert_timeline_contains(&completed, "redaction_status=redacted");
    let trace = completed["trace"]["events"].as_array().unwrap();
    assert!(
        trace
            .iter()
            .any(|event| { event["kind"] == "terminal_observed" && event["success"] == true })
    );
}

#[test]
fn denied_approval_closes_the_persisted_pending_action() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.write_file","arguments":{"path":"denied.md","content":"blocked"}}"#,
    );
    let blocked = create_session_with(&mut router, json!({"initialPrompt":"create denied.md"}));
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"deny"}"#,
    );
    let denied = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &json!({"executionBackendId":"backend.ollama","prompt":"continue","approvalId":approval_id}).to_string(),
    );

    assert_eq!(denied["state"], "blocked", "{denied}");
    assert!(denied["pendingApprovals"].as_array().unwrap().is_empty());
}

fn create_session_with(router: &mut LocalApiRouter, mut body: Value) -> Value {
    body["workspaceId"] =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    body["executionBackendId"] = Value::String("backend.ollama".to_string());
    route_json(router, "POST", "/v1/sessions", &body.to_string())
}

#[test]
fn agent_test_runner_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_test_runner.rs",
        include_str!("local_api_agent_test_runner.rs"),
        155,
    )
    .expect("agent test runner test should stay focused");
}

fn approve(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["message"]
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
