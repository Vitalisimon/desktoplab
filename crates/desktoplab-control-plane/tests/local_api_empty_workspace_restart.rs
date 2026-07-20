use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn restart_does_not_create_a_session_before_the_first_prompt() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let database = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("repo");
    create_repo(&workspace_root);

    let mut router = LocalApiRouter::with_storage_path(&database).expect("router should open");
    mark_setup_ready(&mut router);
    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    let workspace_id = workspace["workspaceId"].as_str().unwrap();
    assert!(sessions(&mut router, workspace_id).is_empty());
    drop(router);

    let mut restarted =
        LocalApiRouter::with_storage_path(&database).expect("router should restart");
    assert!(sessions(&mut restarted, workspace_id).is_empty());
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");
    assert_eq!(state["routeInput"]["activeSessionCount"], 0);
    let workbench = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert!(workbench["session"].is_null(), "{workbench}");
}

#[test]
fn empty_workspace_restart_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_empty_workspace_restart.rs",
        include_str!("local_api_empty_workspace_restart.rs"),
        110,
    )
    .expect("empty workspace restart test should stay focused");
}

fn sessions(router: &mut LocalApiRouter, workspace_id: &str) -> Vec<Value> {
    route_json(
        router,
        "GET",
        &format!("/v1/sessions?workspace_id={workspace_id}"),
        "",
    )["sessions"]
        .as_array()
        .cloned()
        .unwrap()
}

fn create_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .unwrap();
    assert!(status.success());
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
