use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn backend_readiness_starts_blocked_and_tracks_selection() {
    let mut router = LocalApiRouter::default();

    let initial = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(initial["readiness"]["evidence"]["state"], "blocked");
    assert_eq!(
        initial["readiness"]["evidence"]["blockedReason"],
        "runtime_and_model_not_verified"
    );

    let accepted = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    assert_eq!(accepted["readinessEvidence"]["runtimeId"], "runtime.ollama");
    assert_eq!(
        accepted["readinessEvidence"]["modelId"],
        "model.gemma4-12b-q4"
    );
    assert_eq!(accepted["readinessEvidence"]["state"], "blocked");
    assert_eq!(accepted["pipeline"]["state"], "runtime_installing");
    assert_eq!(accepted["startedJobIds"].as_array().unwrap().len(), 2);
    assert_eq!(accepted["jobs"][1]["kind"], "model.download");
    assert_eq!(accepted["jobs"][1]["state"], "blocked");
}

#[test]
fn backend_readiness_is_ready_only_after_runtime_and_model_verification() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    router.mark_runtime_verified_for_test("runtime.ollama", "ollama --version");
    let runtime_only = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(runtime_only["readiness"]["evidence"]["state"], "blocked");
    assert_eq!(
        runtime_only["readiness"]["evidence"]["blockedReason"],
        "model_not_verified"
    );

    router.mark_model_verified_for_test(
        "runtime.ollama",
        "model.gemma4-12b-q4",
        "ollama list gemma4:12b",
    );
    let ready = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(ready["readiness"]["evidence"]["state"], "ready");
    assert_eq!(
        ready["readiness"]["evidence"]["lastEvidence"],
        "ollama list gemma4:12b"
    );
}

#[test]
fn setup_complete_ignores_client_supplied_readiness_flags() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    let forced = route_json(
        &mut router,
        "POST",
        "/v1/setup/complete",
        r#"{"runtimeReady":true,"modelReady":true}"#,
    );

    assert_eq!(forced["setup"]["state"], "blocked");
    assert_eq!(forced["readiness"]["state"], "blocked");
    assert_eq!(
        forced["readiness"]["evidence"]["blockedReason"],
        "runtime_and_model_not_verified"
    );
}

#[test]
fn readiness_state_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/readiness_state.rs",
        include_str!("../src/readiness_state.rs"),
        220,
    )
    .expect("readiness state should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
