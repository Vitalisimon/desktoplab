use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn stream_request_uses_real_agent_path_without_synthetic_delta() {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );
    router.complete_agent_backend_for_test("Streaming response that should be stopped.");

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect slowly with streaming","stream":true}"#,
    );
    let session_id = created["sessionId"].as_str().expect("session id");

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay_payloads(&replay);
    assert!(
        payloads.contains(r#""kind":"agent.stream.started""#),
        "{payloads}"
    );
    assert!(
        !payloads.contains(r#""kind":"agent.stream.delta""#),
        "{payloads}"
    );
    assert_eq!(created["state"], "completed");

    let cancelled = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/control"),
        r#"{"action":"cancel"}"#,
    );
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(cancelled["sessionId"], session_id);

    let sessions = route_json(&mut router, "GET", "/v1/sessions", "");
    assert_eq!(sessions["sessions"][0]["state"], "cancelled");
    assert_eq!(sessions["sessions"][0]["controls"]["cancel"], false);

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay_payloads(&replay);
    assert!(
        payloads.contains(r#""kind":"agent.stream.cancelled""#),
        "{payloads}"
    );
}

#[test]
fn local_api_agent_streaming_cancel_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_streaming_cancel.rs",
        include_str!("local_api_agent_streaming_cancel.rs"),
        120,
    )
    .expect("agent streaming cancel test should stay focused");
}

fn replay_payloads(replay: &Value) -> String {
    replay["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    let _ = route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    let _ = route_json(router, "POST", "/v1/setup/complete", "{}");
}
