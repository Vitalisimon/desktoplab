use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn event_replay_survives_router_restart_with_same_sequences_and_payloads() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    let before = {
        let mut router = LocalApiRouter::with_storage_path(&database).unwrap();
        route_json(
            &mut router,
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
        );
        route_json(&mut router, "GET", "/v1/events/replay", "")["frames"].clone()
    };

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    let after = route_json(&mut restarted, "GET", "/v1/events/replay", "")["frames"].clone();

    assert!(!before.as_array().unwrap().is_empty());
    assert_eq!(after, before);
}

#[test]
fn event_outbox_restart_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_event_outbox_restart.rs",
        include_str!("local_api_event_outbox_restart.rs"),
        90,
    )
    .unwrap();
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
