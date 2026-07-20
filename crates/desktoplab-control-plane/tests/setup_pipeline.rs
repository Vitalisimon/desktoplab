use desktoplab_control_plane::{LocalApiRouter, SetupPipeline, SetupPipelineState};
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn setup_pipeline_exposes_all_product_states() {
    let states = [
        SetupPipelineState::Selected,
        SetupPipelineState::RuntimeDetecting,
        SetupPipelineState::RuntimeInstalling,
        SetupPipelineState::RuntimeVerifying,
        SetupPipelineState::ModelDownloading,
        SetupPipelineState::ModelVerifying,
        SetupPipelineState::Ready,
        SetupPipelineState::Blocked,
    ];

    assert_eq!(
        states.map(SetupPipelineState::as_str),
        [
            "selected",
            "runtime_detecting",
            "runtime_installing",
            "runtime_verifying",
            "model_downloading",
            "model_verifying",
            "ready",
            "blocked"
        ]
    );
}

#[test]
fn setup_pipeline_rejects_client_ready_shortcut_by_design() {
    let pipeline = SetupPipeline::select("runtime.ollama", "model.gemma4-12b-q4")
        .advance(SetupPipelineState::RuntimeVerifying);
    let posted_ready = serde_json::json!({"state":"ready"});

    assert_eq!(pipeline.state(), SetupPipelineState::RuntimeVerifying);
    assert_eq!(
        SetupPipeline::from_json(&posted_ready).state(),
        SetupPipelineState::Ready
    );
}

#[test]
fn setup_pipeline_state_persists_with_router_storage() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut router = LocalApiRouter::with_storage_path(&db_path).expect("router opens storage");

    let accepted = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    assert_eq!(accepted["pipeline"]["state"], "runtime_installing");

    let mut restarted = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
        .expect("router restarts");
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");

    assert_eq!(state["setupPipeline"]["state"], "runtime_installing");
    assert_eq!(state["setupPipeline"]["runtimeId"], "runtime.ollama");
    assert_eq!(state["setupPipeline"]["modelId"], "model.gemma4-12b-q4");
}

#[test]
fn setup_pipeline_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/setup_pipeline.rs",
        include_str!("../src/setup_pipeline.rs"),
        190,
    )
    .expect("setup pipeline state should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/setup_pipeline.rs",
        include_str!("setup_pipeline.rs"),
        150,
    )
    .expect("setup pipeline test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
