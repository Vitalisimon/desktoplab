use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn setup_state_requires_runtime_and_model_before_ready() {
    let mut router = LocalApiRouter::default();

    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(state["setup"]["state"], "not_started");
    assert_ne!(state["readiness"]["state"], "ready");

    let accepted = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    assert_eq!(accepted["setup"]["state"], "in_progress");
    assert_eq!(accepted["pipeline"]["state"], "runtime_installing");
    assert_eq!(accepted["pipeline"]["runtimeId"], "runtime.ollama");
    assert_eq!(accepted["pipeline"]["modelId"], "model.gemma4-12b-q4");
    assert_eq!(accepted["startedJobIds"].as_array().unwrap().len(), 2);
    assert_eq!(accepted["jobs"][0]["state"], "running");
    assert_eq!(accepted["jobs"][1]["state"], "blocked");
    assert_eq!(accepted["jobs"][1]["blockedReason"], "runtime_not_ready");

    let incomplete = route_json(
        &mut router,
        "POST",
        "/v1/setup/complete",
        r#"{"runtimeReady":true,"modelReady":false}"#,
    );
    assert_eq!(incomplete["setup"]["state"], "blocked");
    assert_eq!(incomplete["setupPipeline"]["state"], "blocked");
    assert_eq!(
        incomplete["setupPipeline"]["blockedReason"],
        "runtime_and_model_not_verified"
    );
    assert_eq!(incomplete["readiness"]["state"], "blocked");

    let ready = route_json(
        &mut router,
        "POST",
        "/v1/setup/complete",
        r#"{"runtimeReady":true,"modelReady":true}"#,
    );
    assert_eq!(ready["setup"]["state"], "blocked");
    assert_eq!(ready["setupPipeline"]["state"], "blocked");
    assert_eq!(ready["readiness"]["state"], "blocked");
    assert_eq!(
        ready["readiness"]["evidence"]["blockedReason"],
        "runtime_and_model_not_verified"
    );
}

#[test]
fn setup_complete_derives_ready_after_runtime_and_model_verification() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);

    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    let blocked = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    assert_eq!(blocked["setup"]["state"], "blocked");
    assert_eq!(
        blocked["setupPipeline"]["blockedReason"],
        "model_not_verified"
    );

    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    let ready = route_json(&mut router, "POST", "/v1/setup/complete", "{}");

    assert_eq!(ready["setup"]["state"], "ready");
    assert_eq!(ready["setupPipeline"]["state"], "ready");
    assert_eq!(ready["readiness"]["state"], "ready");
}

#[test]
fn setup_accept_is_idempotent_after_setup_is_ready() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);

    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    let _ = route_json(&mut router, "POST", "/v1/setup/complete", "{}");

    let accepted_again = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    assert_eq!(accepted_again["setup"]["state"], "ready");
    assert_eq!(accepted_again["pipeline"]["state"], "ready");
    assert_eq!(accepted_again["jobs"].as_array().unwrap().len(), 0);
    assert_eq!(accepted_again["startedJobIds"].as_array().unwrap().len(), 0);
}

#[test]
fn setup_accept_rejects_empty_selection() {
    let mut router = LocalApiRouter::default();

    let response = router
        .route("POST", "/v1/setup/accept", r#"{"runtimeId":""}"#)
        .expect("route should exist");

    assert_eq!(response.status(), "400 Bad Request");
}

#[test]
fn setup_state_sources_stay_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/setup_state.rs",
        include_str!("../src/setup_state.rs"),
        220,
    )
    .expect("setup state should stay focused");
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/app_state.rs",
        include_str!("../src/app_state.rs"),
        80,
    )
    .expect("app state projection should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
