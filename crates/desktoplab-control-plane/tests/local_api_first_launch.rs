use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn fresh_app_state_enters_setup_without_workspace() {
    let mut router = LocalApiRouter::default();

    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "not_started");
    assert_eq!(state["readiness"]["state"], "blocked");
    assert_eq!(state["routeInput"]["readiness"], "blocked");
    assert_eq!(state["routeInput"]["hasWorkspace"], false);
    assert!(state["currentWorkspace"].is_null());
}

#[test]
fn workspace_open_is_rejected_until_setup_is_ready() {
    let mut router = LocalApiRouter::default();

    let rejected = route_json_with_status(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        r#"{"path":"/tmp/desktoplab"}"#,
        "400 Bad Request",
    );
    assert_eq!(rejected["code"], "SETUP_REQUIRED");
    assert_eq!(rejected["blockedReason"], "setup_not_ready");

    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "not_started");
    assert_eq!(state["readiness"]["state"], "blocked");
    assert_eq!(state["routeInput"]["readiness"], "blocked");
    assert_eq!(state["routeInput"]["hasWorkspace"], false);
    assert!(state["currentWorkspace"].is_null());
}

#[test]
fn workspace_open_is_allowed_after_setup_is_ready() {
    let workspace_fixture = git_workspace_fixture();
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);

    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_fixture.path()),
    );

    assert_eq!(
        workspace["rootPath"],
        workspace_fixture.path().display().to_string()
    );
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(state["setup"]["state"], "ready");
    assert_eq!(state["routeInput"]["hasWorkspace"], true);
}

#[test]
fn persisted_ready_setup_without_backend_readiness_is_blocked() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let store = desktoplab_storage::SqliteStore::open(&db_path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
        .put_productization_state(desktoplab_storage::ProductizationStateRecord::new(
            desktoplab_storage::ProductizationRecordKind::SetupState,
            "local",
            r#"{"state":"ready","runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4","runtimeReady":true,"modelReady":true}"#,
        ))
        .expect("legacy ready setup should persist");

    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
        .expect("router should resume");
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(state["setup"]["state"], "blocked");
    assert_eq!(state["readiness"]["state"], "blocked");
    assert_eq!(
        state["readiness"]["evidence"]["blockedReason"],
        "runtime_and_model_not_verified"
    );

    let rejected = route_json_with_status(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        r#"{"path":"/tmp/desktoplab"}"#,
        "400 Bad Request",
    );
    assert_eq!(rejected["blockedReason"], "setup_not_ready");
}

#[test]
fn setup_state_survives_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    {
        let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
            .expect("router should open");
        route_json(
            &mut router,
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
        );
    }

    let mut resumed = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db_path)
        .expect("router should resume");
    let state = route_json(&mut resumed, "GET", "/v1/app/state", "");

    assert_eq!(state["setup"]["state"], "in_progress");
    assert_eq!(state["setup"]["runtimeId"], "runtime.ollama");
    assert_eq!(state["setup"]["modelId"], "model.gemma4-12b-q4");
    assert_eq!(state["readiness"]["state"], "blocked");
}

fn git_workspace_fixture() -> TempDir {
    let fixture = TempDir::new().expect("workspace fixture should exist");
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(fixture.path())
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
    fixture
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    route_json_with_status(router, method, path, body, "200 OK")
}

fn route_json_with_status(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
    status: &str,
) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), status, "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
