use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn current_workspace_survives_router_restart_through_storage() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("repo");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init"]);

    let store = SqliteStore::open(&db_path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    let mut first_router = LocalApiRouter::with_storage(store).expect("router should open");
    mark_setup_ready(&mut first_router);
    route_json(
        &mut first_router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );

    let resumed_store = SqliteStore::open(&db_path).expect("store should reopen");
    resumed_store
        .apply_migrations()
        .expect("migrations should be idempotent");
    let mut resumed_router =
        LocalApiRouter::with_storage(resumed_store).expect("router should resume");
    let state = route_json(&mut resumed_router, "GET", "/v1/app/state", "");

    assert_eq!(state["readiness"]["state"], "ready");
    assert_eq!(state["setup"]["state"], "ready");
    assert_eq!(state["currentWorkspace"]["workspaceId"], "workspace.repo");
    assert_eq!(
        state["currentWorkspace"]["rootPath"].as_str(),
        Some(workspace_root.to_string_lossy().as_ref())
    );
    assert_eq!(state["routeInput"]["hasWorkspace"], true);
}

#[test]
fn workspace_open_trims_trailing_separator_before_deriving_identity() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace_root = fixture.path().join("ContactCreator");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let path_with_trailing_separator = format!("{}/", workspace_root.display());

    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &format!(r#"{{"path":"{path_with_trailing_separator}"}}"#),
    );

    assert_eq!(workspace["workspaceId"], "workspace.ContactCreator");
    assert_eq!(workspace["displayName"], "ContactCreator");
    assert_eq!(
        workspace["rootPath"],
        workspace_root.to_string_lossy().as_ref()
    );
}

#[test]
fn stale_empty_workspace_identity_is_repaired_on_load() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("ContactCreator");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init"]);
    let store = SqliteStore::open(&db_path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::CurrentWorkspace,
            "current",
            format!(
                r#"{{"workspaceId":"workspace.","displayName":"","rootPath":"{}/"}}"#,
                workspace_root.display()
            ),
        ))
        .expect("stale workspace should persist");

    let mut router = LocalApiRouter::with_storage_path(&db_path).expect("router should load");
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(
        state["currentWorkspace"]["workspaceId"],
        "workspace.ContactCreator"
    );
    assert_eq!(state["currentWorkspace"]["displayName"], "ContactCreator");
    assert_eq!(
        state["currentWorkspace"]["rootPath"],
        workspace_root.to_string_lossy().as_ref()
    );
}

#[test]
fn workspace_open_requires_existing_git_repository() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let non_repo = fixture.path().join("not-a-repo");
    std::fs::create_dir_all(&non_repo).expect("directory should exist");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let response = router
        .route(
            "POST",
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&non_repo),
        )
        .expect("workspace open should route");
    assert_eq!(response.status(), "400 Bad Request");
    let body: Value = serde_json::from_str(response.body()).expect("body should be json");
    assert_eq!(body["code"], "GIT_REPOSITORY_REQUIRED");
}

#[test]
fn workspace_open_can_initialize_git_after_explicit_user_choice() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let project = fixture.path().join("new-project");
    std::fs::create_dir_all(&project).expect("directory should exist");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&project),
    );

    assert_eq!(workspace["workspaceId"], "workspace.new-project");
    assert_eq!(workspace["displayName"], "new-project");
    assert!(project.join(".git").is_dir());
}

#[test]
fn workspace_open_requires_existing_folder_before_workbench() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let missing = fixture.path().join("missing-repo");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let response = router
        .route(
            "POST",
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&missing),
        )
        .expect("workspace open should route");
    assert_eq!(response.status(), "400 Bad Request");
    let body: Value = serde_json::from_str(response.body()).expect("body should be json");
    assert_eq!(body["code"], "WORKSPACE_PATH_NOT_FOUND");
}

#[test]
fn workspace_state_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_workspace_state.rs",
        include_str!("local_api_workspace_state.rs"),
        210,
    )
    .expect("workspace state test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/setup_runtime_model/workspace_open.rs",
        include_str!("../src/router/setup_runtime_model/workspace_open.rs"),
        120,
    )
    .expect("workspace open router should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
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

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
