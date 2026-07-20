use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde_json::Value;

#[test]
fn runtime_install_job_state_survives_router_restart() {
    let db = tempfile::NamedTempFile::new().expect("temp sqlite file");
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    assert_eq!(install["source"], "service_backed");
    let job_id = install["jobId"].as_str().expect("job id");
    assert!(job_id.starts_with("job."));

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    let diagnostics = route_json(&mut restarted, "GET", "/v1/diagnostics", "");

    assert!(
        diagnostics["bundlePreview"]["summary"]
            .as_str()
            .expect("diagnostic summary")
            .contains("runtime.install:"),
        "{diagnostics}"
    );
}

#[test]
fn stale_running_runtime_setup_without_host_recovery_blocks_on_restart() {
    let db = tempfile::NamedTempFile::new().expect("temp sqlite file");
    let store = SqliteStore::open(db.path()).expect("store opens");
    store.apply_migrations().expect("migrations apply");
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::SetupState,
            "local",
            r#"{"state":"in_progress","runtimeId":"runtime.lm-studio","modelId":"model.gemma4-12b-q4","runtimeReady":false,"modelReady":false}"#,
        ))
        .expect("setup persists");
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::SetupPipeline,
            "local",
            r#"{"state":"runtime_installing","runtimeId":"runtime.lm-studio","modelId":"model.gemma4-12b-q4","blockedReason":null}"#,
        ))
        .expect("pipeline persists");
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::RuntimeJob,
            "runtime.install",
            r#"{"jobs":[{"id":"job.1","kind":"runtime.install","state":"running"},{"id":"job.3","kind":"runtime.install","state":"failed"}]}"#,
        ))
        .expect("runtime jobs persist");

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");

    assert_eq!(state["setupPipeline"]["state"], "blocked");
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["state"],
        "blocked"
    );
}

#[test]
fn blocked_setup_recovers_when_existing_host_runtime_and_model_are_verified() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64}"#,
    );

    router.reconcile_existing_host_setup_for_test(true, true);
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "ready");
    assert_eq!(state["setupPipeline"]["state"], "ready");
    assert_eq!(state["readiness"]["state"], "ready");
}

#[test]
fn existing_runtime_recovery_clears_stale_permission_pipeline_when_model_is_missing() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64}"#,
    );

    router.reconcile_existing_host_setup_for_test(true, false);
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "blocked");
    assert_eq!(state["setup"]["blockedReason"], "model_not_verified");
    assert_eq!(state["setupPipeline"]["state"], "selected");
    assert_eq!(state["setupPipeline"]["blockedReason"], Value::Null);
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["state"],
        "verified"
    );
    assert_eq!(
        state["readiness"]["evidence"]["modelVerification"]["state"],
        "missing"
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
