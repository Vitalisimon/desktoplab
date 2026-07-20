use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn malformed_workspace_state_does_not_create_a_current_or_registered_workspace() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let database = fixture.path().join("desktoplab.sqlite");
    let store = prepared_store(&database);
    store
        .put_productization_states(&[
            ProductizationStateRecord::new(
                ProductizationRecordKind::CurrentWorkspace,
                "current",
                json!({"workspaceId":"workspace.desktoplab"}).to_string(),
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::WorkspaceRegistry,
                "local",
                json!({
                    "workspaces":[{"displayName":"desktoplab"}],
                    "archivedWorkspaceIds":[],
                    "archivedSessionIds":[]
                })
                .to_string(),
            ),
        ])
        .expect("malformed fixtures should persist");

    let mut router = LocalApiRouter::with_storage_path(&database).expect("router should load");
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    let workspaces = route_json(&mut router, "GET", "/v1/workspaces", "");

    assert_eq!(state["currentWorkspace"], Value::Null);
    assert_eq!(state["routeInput"]["hasWorkspace"], false);
    assert!(workspaces["workspaces"].as_array().unwrap().is_empty());
}

#[test]
fn orphaned_memory_cannot_resurrect_or_mutate_an_unregistered_workspace() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let database = fixture.path().join("desktoplab.sqlite");
    let store = prepared_store(&database);
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::WorkspaceMemory,
            "workspace.desktoplab",
            json!({"memories":[{
                "memoryId":"workspace.desktoplab:memory.1",
                "workspaceId":"workspace.desktoplab",
                "kind":"repo_summary",
                "title":"Ghost",
                "summary":"Synthetic state",
                "decisions":[],
                "source":"fixture",
                "createdAt":"1970-01-01T00:00:00Z"
            }]})
            .to_string(),
        ))
        .expect("orphaned memory should persist");

    let mut router = LocalApiRouter::with_storage_path(&database).expect("router should load");
    assert_status(
        &mut router,
        "GET",
        "/v1/workspaces/workspace.desktoplab/memory",
        "",
        "404 Not Found",
    );
    assert_status(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.desktoplab/memory",
        r#"{"title":"New ghost"}"#,
        "404 Not Found",
    );
}

#[test]
fn runtime_workspace_loading_contains_no_synthetic_identity_or_root() {
    let sources = [
        include_str!("../src/router/persistence_load.rs"),
        include_str!("../src/router/setup_runtime_model/workspace_open.rs"),
    ]
    .join("\n");

    assert!(!sources.contains("workspace.desktoplab"));
    assert!(!sources.contains("/repo/desktoplab"));
}

#[test]
fn workspace_open_requires_an_explicit_nonempty_path() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let response = router
        .route("POST", "/v1/workspaces/open", "{}")
        .expect("workspace open should route");
    assert_eq!(response.status(), "400 Bad Request");
    let body: Value = serde_json::from_str(response.body()).expect("response should be json");
    assert_eq!(body["code"], "WORKSPACE_PATH_NOT_FOUND");
    assert_eq!(body["blockedReason"], "workspace_path_not_found");
}

#[test]
fn workspace_persistence_truth_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_workspace_persistence_truth.rs",
        include_str!("local_api_workspace_persistence_truth.rs"),
        170,
    )
    .expect("workspace persistence truth test should stay focused");
}

fn prepared_store(database: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(database).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
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
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn assert_status(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
    expected: &str,
) {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), expected, "{}", response.body());
}
