use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn product_session_runs_read_approval_write_and_completion_in_one_native_loop() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    std::fs::write(workspace.path().join("README.md"), "# Native history\n").unwrap();
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"proof.md","content":"native loop\n"}}"#,
        r#"{"id":"read-2","tool":"desktoplab.read_file","arguments":{"path":"proof.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Created proof.md after inspecting README.md.","outcome":"changed","evidenceCallIds":["read-1","write-1","read-2"]}}"#,
    ]);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Inspect README.md, then create proof.md."}}"#
        ),
    );

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert!(!workspace.path().join("proof.md").exists());
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let workspace_state = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace_state["session"]["state"], "completed");
    assert_eq!(
        workspace_state["session"]["summary"],
        "Created proof.md after inspecting README.md."
    );
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("proof.md")).unwrap(),
        "native loop\n"
    );
    let timeline = workspace_state["session"]["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message == "Read README.md:\n# Native history")
        }),
        "the executor read evidence should be preserved for the UI: {timeline:?}"
    );
    assert!(
        timeline.iter().any(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message == "Read proof.md:\nnative loop")
        }),
        "the completed mutation should retain its readback evidence: {timeline:?}"
    );
    assert_eq!(
        timeline
            .iter()
            .filter(|event| {
                event["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("desktoplab.write_file"))
            })
            .count(),
        4,
        "the write should have one planned, approved, executed and observed lifecycle: {timeline:?}"
    );
}

#[test]
fn model_plan_updates_replace_the_prompt_placeholder_with_durable_task_state() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"plan-1","tool":"desktoplab.update_plan","arguments":{"steps":[{"step":"Inspect repository","status":"completed"},{"step":"Implement fix","status":"in_progress"},{"step":"Run tests","status":"pending"}]}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Plan recorded.","outcome":"executed","evidenceCallIds":["plan-1"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"fix the issue"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        completed["plan"],
        "[completed] Inspect repository\n[in_progress] Implement fix\n[pending] Run tests"
    );
}

#[test]
fn model_clarification_blocks_the_native_session_without_fake_execution() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.clarify","arguments":{"question":"Which file should I inspect?","blockedOn":"desktoplab.read_file"}}"#,
    ]);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Inspect the requested file."}}"#
        ),
    );

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_eq!(blocked["pendingApprovals"], serde_json::json!([]));
    assert!(
        blocked["timeline"]
            .to_string()
            .contains("clarification_required:Which file should I inspect?"),
        "{blocked}"
    );
    assert!(!blocked["timeline"].to_string().contains("state=observed"));
}

#[test]
fn repeated_malformed_tool_names_get_two_bounded_protocol_retries() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"read_file","arguments":{"path":"README.md"}}"#,
        r#"{"tool":"read_file","arguments":{"path":"README.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Recovered through the canonical protocol.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Answer without repository evidence."}}"#
        ),
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        completed["summary"],
        "Recovered through the canonical protocol."
    );
}

#[test]
fn native_iterative_product_test_stays_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-control-plane/tests/local_api_native_iterative_loop.rs",
            include_str!("local_api_native_iterative_loop.rs"),
            240,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_iterative.rs",
            include_str!("../src/router/agent_iterative.rs"),
            380,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_plan_tools.rs",
            include_str!("../src/router/agent_plan_tools.rs"),
            160,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit)
            .expect("native iterative source should stay focused");
    }
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b    5.2 GB"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b    5.2 GB"}"#,
    );
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    serde_json::from_str(response.body()).expect("route response json")
}

fn create_repo(path: &std::path::Path) {
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .expect("git init should run");
    assert!(status.success());
}
