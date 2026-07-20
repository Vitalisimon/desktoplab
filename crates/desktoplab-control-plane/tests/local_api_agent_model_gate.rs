use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{
    ProductizationRecordKind, ProductizationStateRecord, SettingRecord, SettingValue, SqliteStore,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn installed_model_without_certified_tool_protocol_is_not_an_agent_route() {
    let (_fixture, mut router, _workspace_id) = ready_workspace_without_agent_protocol();

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace["route"]["status"], "blocked");
    assert_eq!(
        workspace["route"]["blockedReasons"][0],
        "model_tool_protocol_uncertified"
    );
}

#[test]
fn prompt_is_rejected_before_an_uncertified_model_reaches_the_executor() {
    let (_fixture, mut router, workspace_id) = ready_workspace_without_agent_protocol();
    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &serde_json::json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Inspect the repository"
        })
        .to_string(),
    );

    assert_eq!(session["accepted"], false);
    assert_eq!(session["state"], "blocked");
    assert_eq!(session["blockedReason"], "model_tool_protocol_uncertified");
}

#[test]
fn removed_catalog_model_is_invalidated_when_persisted_state_reopens() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    seed_removed_model_state(&database);

    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database)
        .expect("router should reopen");
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    let routes = route_json(&mut router, "GET", "/v1/routing/options", "");

    assert_eq!(state["setup"]["state"], "not_started");
    assert_eq!(state["readiness"]["state"], "blocked");
    assert_eq!(state["readiness"]["evidence"]["modelId"], Value::Null);
    assert_eq!(routes["selectedRouteId"], "route.local.unconfigured");
}

#[test]
fn route_options_include_installed_candidates_without_prevalidating_them() {
    let mut router = LocalApiRouter::default();
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama ready");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma installed");
    router.set_local_model_inventory_for_test(&["gemma4:12b", "qwen3.5:9b"]);

    let routes = route_json(&mut router, "GET", "/v1/routing/options", "");
    let local = routes["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|route| route["backendKind"] == "local")
        .collect::<Vec<_>>();
    let ids = local
        .iter()
        .filter_map(|route| route["modelId"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["model.gemma4-12b-q4", "model.qwen3.5-9b-q4"]);
    assert!(local.iter().all(|route| route["status"] == "available"));
}

fn ready_workspace_without_agent_protocol() -> (TempDir, LocalApiRouter, String) {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    router.set_local_model_inventory_for_test(&["gemma4:12b"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama ready");
    router.mark_model_verified_without_capabilities_for_test(
        "runtime.ollama",
        "model.gemma4-12b-q4",
        "gemma installed",
    );
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    let workspace_id = workspace["workspaceId"].as_str().unwrap().to_string();
    (fixture, router, workspace_id)
}

fn seed_removed_model_state(path: &std::path::Path) {
    let store = SqliteStore::open(path).unwrap();
    store.apply_migrations().unwrap();
    store
        .put_productization_states(&[
            ProductizationStateRecord::new(
                ProductizationRecordKind::SetupState,
                "local",
                r#"{"state":"ready","runtimeId":"runtime.ollama","modelId":"model.gemma3-12b-q4","runtimeReady":true,"modelReady":true}"#,
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::SetupPipeline,
                "local",
                r#"{"state":"ready","runtimeId":"runtime.ollama","modelId":"model.gemma3-12b-q4"}"#,
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::BackendReadiness,
                "local",
                r#"{"state":"ready","runtimeId":"runtime.ollama","modelId":"model.gemma3-12b-q4","runtimeVerification":{"state":"verified"},"modelVerification":{"state":"verified"}}"#,
            ),
        ])
        .unwrap();
    store
        .put_setting(SettingRecord::new(
            "routing.selected_route_id",
            SettingValue::String("route.local.gemma3-12b-q4".to_string()),
        ))
        .unwrap();
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
