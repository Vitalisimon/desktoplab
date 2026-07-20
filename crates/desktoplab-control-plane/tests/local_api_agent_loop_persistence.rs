use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};
use xtask::check_logical_line_limit;

#[test]
fn running_streaming_agent_session_recovers_blocked_and_can_be_cancelled() {
    let db = NamedTempFile::new().expect("temp sqlite file");
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    mark_setup_ready(&mut router);
    let workspace_record = open_workspace(&mut router, workspace.path());
    let workspace_id = workspace_record["workspaceId"].as_str().unwrap();

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"stream and wait","stream":true}}"#
        ),
    );
    let session_id = created["sessionId"].as_str().unwrap().to_string();
    assert_eq!(created["state"], "running");
    assert_eq!(
        router
            .agent_execution_binding_for_test(&session_id)
            .unwrap()["modelId"],
        "model.gemma4-12b-q4"
    );
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    assert_eq!(
        restarted
            .agent_execution_binding_for_test(&session_id)
            .unwrap()["modelId"],
        "model.gemma4-12b-q4"
    );
    let listed = route_json(&mut restarted, "GET", "/v1/sessions", "");
    assert_eq!(listed["sessions"][0]["sessionId"], session_id);
    assert_eq!(listed["sessions"][0]["state"], "blocked");
    assert_eq!(
        listed["sessions"][0]["blockedReason"],
        "long_running_job_interrupted"
    );
    assert_eq!(listed["sessions"][0]["controls"]["cancel"], true);

    let cancelled = route_json(
        &mut restarted,
        "POST",
        &format!("/v1/sessions/{session_id}/control"),
        r#"{"action":"cancel"}"#,
    );
    assert_eq!(cancelled["sessionId"], session_id);
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(cancelled["controls"]["resume"], false);
}

#[test]
fn local_api_agent_loop_persistence_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_loop_persistence.rs",
        include_str!("local_api_agent_loop_persistence.rs"),
        130,
    )
    .expect("agent loop persistence test should stay focused");
}

fn open_workspace(router: &mut LocalApiRouter, path: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    )
}

fn create_repo(path: &std::path::Path) {
    let output = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .expect("git init should run");
    assert!(output.status.success(), "git init failed");
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
