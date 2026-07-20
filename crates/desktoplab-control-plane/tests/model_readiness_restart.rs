use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn verified_model_readiness_survives_router_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut first_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should open storage");

    mark_setup_ready(&mut first_router);

    let mut restarted =
        LocalApiRouter::with_storage_path(&db_path).expect("router should reopen storage");
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "ready");
    assert_eq!(state["readiness"]["state"], "ready");
    assert_eq!(
        state["readiness"]["evidence"]["modelVerification"]["state"],
        "verified"
    );
}

#[test]
fn missing_model_inventory_blocks_restart_readiness() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut first_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should open storage");

    mark_setup_ready(&mut first_router);

    let mut restarted =
        LocalApiRouter::with_storage_path(&db_path).expect("router should reopen storage");
    restarted.set_local_model_inventory_for_test(&["llama3.1:8b"]);
    let missing = route_json(
        &mut restarted,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        "{}",
    );

    assert_eq!(missing["verificationState"], "blocked");
    assert_eq!(missing["blockedReason"], "model_not_reported_by_runtime");

    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");
    assert_eq!(state["setup"]["state"], "blocked");
    assert_eq!(state["readiness"]["state"], "blocked");
    assert_eq!(
        state["readiness"]["evidence"]["modelVerification"]["state"],
        "blocked"
    );
}

#[test]
fn model_readiness_restart_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/model_readiness_restart.rs",
        include_str!("model_readiness_restart.rs"),
        150,
    )
    .expect("model readiness restart test should stay focused");
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
