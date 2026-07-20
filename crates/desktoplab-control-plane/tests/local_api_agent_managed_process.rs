use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn product_loop_starts_and_controls_same_session_owned_process() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        json!({
            "id":"start-1",
            "tool":"desktoplab.start_process",
            "arguments":{"command":slow_command()}
        })
        .to_string(),
        json!({
            "id":"kill-1",
            "tool":"desktoplab.kill_process",
            "arguments":{"processId":"process.1"}
        })
        .to_string(),
        json!({
            "tool":"desktoplab.complete",
            "arguments":{
                "message":"Started and stopped the managed process.",
                "outcome":"executed",
                "evidenceCallIds":["start-1","kill-1"]
            }
        })
        .to_string(),
    ]);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Start the background task, inspect its process ID, then stop it."
        })
        .to_string(),
    );
    assert_eq!(blocked["state"], "blocked", "{blocked}");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert_eq!(
        completed["session"]["summary"],
        "Started and stopped the managed process."
    );
    let timeline = completed["session"]["timeline"].to_string();
    assert!(
        timeline
            .contains("state=observed source=agent.iterative canonical=desktoplab.start_process"),
        "{timeline}"
    );
    assert!(
        timeline
            .contains("state=observed source=agent.iterative canonical=desktoplab.kill_process"),
        "{timeline}"
    );
    assert_eq!(
        timeline.matches("waiting for approval").count(),
        1,
        "{timeline}"
    );
}

#[test]
fn product_loop_writes_and_polls_the_same_session_owned_process() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        json!({
            "id":"start-io",
            "tool":"desktoplab.start_process",
            "arguments":{"command":interactive_command()}
        })
        .to_string(),
        json!({
            "id":"stdin-io",
            "tool":"desktoplab.write_process_stdin",
            "arguments":{"processId":"process.1","input":"hello\n"}
        })
        .to_string(),
        json!({
            "id":"poll-io",
            "tool":"desktoplab.poll_process",
            "arguments":{"processId":"process.1"}
        })
        .to_string(),
        json!({
            "tool":"desktoplab.complete",
            "arguments":{
                "message":"Sent input to and observed the managed process.",
                "outcome":"executed",
                "evidenceCallIds":["start-io","stdin-io","poll-io"]
            }
        })
        .to_string(),
    ]);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Start an interactive process, send input, and poll it."
        })
        .to_string(),
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
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let timeline = completed["session"]["timeline"].to_string();

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    for tool in ["start_process", "write_process_stdin", "poll_process"] {
        assert!(
            timeline.contains(&format!("canonical=desktoplab.{tool}")),
            "{timeline}"
        );
    }
}

#[test]
fn managed_process_product_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_managed_process.rs",
        include_str!("local_api_agent_managed_process.rs"),
        260,
    )
    .unwrap();
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
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
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

fn create_repo(path: &std::path::Path) {
    assert!(
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
}

#[cfg(not(windows))]
fn slow_command() -> &'static str {
    "printf ready; sleep 30"
}

#[cfg(not(windows))]
fn interactive_command() -> &'static str {
    "read line; printf 'received:%s' \"$line\""
}

#[cfg(windows)]
fn interactive_command() -> &'static str {
    "$line = [Console]::In.ReadLine(); [Console]::Write(\"received:$line\")"
}

#[cfg(windows)]
fn slow_command() -> &'static str {
    "[Console]::Write('ready'); Start-Sleep -Seconds 30"
}
