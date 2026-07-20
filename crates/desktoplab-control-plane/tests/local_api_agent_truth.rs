use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn agent_workspace_is_blocked_until_setup_ready() {
    let mut router = LocalApiRouter::default();
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace["route"]["status"], "blocked");
    assert_eq!(workspace["session"], Value::Null);
    assert!(
        workspace["route"]["blockedReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str() == Some("setup_not_ready"))
    );
}

#[test]
fn session_creation_requires_selected_backend() {
    let mut router = LocalApiRouter::default();
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect repo"}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        blocked["summary"],
        "Setup must finish before the agent can start."
    );
}

#[test]
fn agent_events_are_replayable_after_blocked_start() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect repo"}"#,
    );
    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");

    assert!(
        replay["frames"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| frame["payload"].as_str().unwrap().contains("agent.blocked"))
    );
}

#[test]
fn agent_truth_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_truth.rs",
        include_str!("local_api_agent_truth.rs"),
        180,
    )
    .expect("agent truth route test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
