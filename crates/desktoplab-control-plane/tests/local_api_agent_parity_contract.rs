use desktoplab_agent_engine::{FailureObservation, RetryAttempt, RetryDecision, RetryPolicy};
use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn explain_repository_prompt_uses_workspace_inspection_tools() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    write_workspace_fixture(&workspace_root);
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Ispeziono il README.","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
    );

    let completed = create_session(&mut router, "spiega questa repo");

    assert_eq!(completed["state"], "completed");
    assert_tool_decision(&completed, "filesystem.read:README.md");
    assert_observation_contains(&completed, "Agent Parity Fixture");
}

#[test]
fn locate_composer_prompt_searches_before_answering() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    write_workspace_fixture(&workspace_root);
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Cerco il composer.","tool":"desktoplab.search_text","arguments":{"query":"AgentComposer","path":""}}"#,
    );

    let completed = create_session(&mut router, "trova dove viene gestito il composer");

    assert_eq!(completed["state"], "completed");
    assert_tool_decision(&completed, "search.text:AgentComposer");
    assert_observation_contains(
        &completed,
        "apps/desktop/src/features/productization/AgentComposer.tsx",
    );
}

#[test]
fn create_edit_validate_and_diff_flow_records_real_agent_evidence() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    write_workspace_fixture(&workspace_root);
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo prova.md.","desktoplabAction":{"kind":"create_file","path":"prova.md","content":"# Prova\n\nNota iniziale.\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea prova.md con una nota");
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "filesystem.write:prova.md"
    );
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed_create = continue_approval(&mut router, &blocked, &approval_id);
    assert_eq!(completed_create["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("prova.md")).unwrap(),
        "# Prova\n\nNota iniziale.\n"
    );

    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Eseguo i test mirati.","tool":"desktoplab.run_tests","arguments":{"command":"printf validation-ok","reason":"validate created file"}}"#,
    );
    let test_blocked = create_session(&mut router, "esegui i test mirati");
    assert_eq!(test_blocked["state"], "blocked");
    assert_eq!(
        test_blocked["pendingApprovals"][0]["operationId"],
        "test.run:printf validation-ok"
    );
    let test_approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &test_approval_id);
    let completed_test = continue_approval(&mut router, &test_blocked, &test_approval_id);
    assert_eq!(completed_test["state"], "completed");
    assert_observation_contains(&completed_test, "validation-ok");

    let retry_policy = RetryPolicy::new(1);
    let failed = FailureObservation::test_failed("validation failed");
    assert_eq!(
        retry_policy.evaluate(&[], &failed).decision(),
        RetryDecision::Retry
    );
    let attempts = [RetryAttempt::from_observation(failed)
        .with_patch_summary("patched prova.md")
        .with_rerun_summary("validation still failed")];
    let stopped = retry_policy.evaluate(
        &attempts,
        &FailureObservation::test_failed("validation still failed"),
    );
    assert_eq!(stopped.decision(), RetryDecision::Stop);
    assert!(stopped.truthful_summary().contains("still failing"));
}

#[test]
fn agent_parity_contract_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_parity_contract.rs",
        include_str!("local_api_agent_parity_contract.rs"),
        260,
    )
    .expect("agent parity contract test should stay focused");
}

fn write_workspace_fixture(workspace_root: &std::path::Path) {
    std::fs::create_dir_all(workspace_root.join("src")).expect("src dir should write");
    std::fs::create_dir_all(workspace_root.join("apps/desktop/src/features/productization"))
        .expect("composer dir should write");
    std::fs::write(
        workspace_root.join("README.md"),
        "# Agent Parity Fixture\n\nThis repository tests DesktopLab agent behavior.\n",
    )
    .expect("README should write");
    std::fs::write(
        workspace_root.join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .expect("lib should write");
    std::fs::write(
        workspace_root.join("apps/desktop/src/features/productization/AgentComposer.tsx"),
        "export function AgentComposer() { return null; }\n",
    )
    .expect("composer should write");
    run_git(workspace_root, &["add", "."]);
    run_git(workspace_root, &["commit", "-m", "initial fixture"]);
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn continue_approval(router: &mut LocalApiRouter, blocked: &Value, approval_id: &str) -> Value {
    route_json(
        router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    )
}

fn latest_approval_id(router: &mut LocalApiRouter) -> String {
    let listed = route_json(router, "GET", "/v1/approvals", "");
    listed["approvals"]
        .as_array()
        .and_then(|approvals| approvals.last())
        .and_then(|approval| approval["approvalId"].as_str())
        .unwrap()
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

fn assert_tool_decision(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| {
            event["kind"] == "tool_decision"
                && event["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected))
        }),
        "missing tool decision {expected}: {session}"
    );
}

fn assert_observation_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| {
            matches!(event["kind"].as_str(), Some("tool" | "assistant"))
                && event["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected))
        }),
        "missing observation {expected}: {session}"
    );
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
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
