use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn approval_modes_are_backend_owned_and_exposed_to_app_state() {
    let mut router = LocalApiRouter::default();

    let modes = route_json(&mut router, "GET", "/v1/approval-modes", "");
    assert_eq!(modes["defaultMode"], "require_approval");
    assert_eq!(modes["sessionMode"], "require_approval");
    assert_eq!(
        mode_ids(&modes),
        [
            "require_approval",
            "approve_for_me",
            "approve_workspace_writes_for_session",
            "full_access"
        ]
    );
    assert_eq!(modes["modes"][0]["label"], "Ask for approval");
    assert_eq!(modes["modes"][1]["label"], "Approve routine actions");
    assert_eq!(modes["modes"][2]["label"], "Allow workspace writes");
    assert_eq!(modes["modes"][3]["label"], "Full local access");

    let updated_default = route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/default",
        r#"{"mode":"approve_for_me"}"#,
    );
    assert_eq!(updated_default["defaultMode"], "approve_for_me");
    assert_eq!(updated_default["sessionMode"], "approve_for_me");

    let updated_session = route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"full_access"}"#,
    );
    assert_eq!(updated_session["defaultMode"], "approve_for_me");
    assert_eq!(updated_session["sessionMode"], "full_access");

    let app_state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(app_state["approvalModes"]["defaultMode"], "approve_for_me");
    assert_eq!(app_state["approvalModes"]["sessionMode"], "full_access");
    assert_eq!(app_state["routeInput"]["approvalMode"], "full_access");
}

#[test]
fn unknown_approval_modes_are_rejected_with_typed_error() {
    let mut router = LocalApiRouter::default();

    let response = router
        .route(
            "POST",
            "/v1/approval-modes/session",
            r#"{"mode":"whatever"}"#,
        )
        .expect("approval mode route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    let body: Value = serde_json::from_str(response.body()).expect("error should be json");
    assert_eq!(body["code"], "INVALID_APPROVAL_MODE");
    assert_eq!(
        body["allowedModes"],
        serde_json::json!([
            "require_approval",
            "approve_for_me",
            "approve_workspace_writes_for_session",
            "full_access"
        ])
    );
}

#[test]
fn default_approval_mode_persists_across_router_reopen() {
    let temp = TempDir::new().expect("temp dir should exist");
    let db = temp.path().join("desktoplab.sqlite");

    {
        let mut router = LocalApiRouter::with_storage_path(&db).expect("router should open");
        route_json(
            &mut router,
            "POST",
            "/v1/approval-modes/default",
            r#"{"mode":"full_access"}"#,
        );
    }

    let mut reopened = LocalApiRouter::with_storage_path(&db).expect("router should reopen");
    let modes = route_json(&mut reopened, "GET", "/v1/approval-modes", "");
    assert_eq!(modes["defaultMode"], "full_access");
    assert_eq!(modes["sessionMode"], "full_access");
}

#[test]
fn session_approval_mode_is_applied_to_agent_tool_execution() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);
    router.complete_agent_backend_for_test("I can update the file.");
    route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"approve_for_me"}"#,
    );

    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","initialPrompt":"Update README","plannedTool":"filesystem.write","toolPath":"README.md"}"#,
    );

    assert_eq!(session["state"], "completed");
    assert!(
        session["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["message"]
                .as_str()
                .unwrap_or_default()
                .contains("approval_mode=approve_for_me")),
        "{session:#?}"
    );
}

#[test]
fn workspace_write_session_mode_scopes_auto_approval_to_file_writes() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);
    router.complete_agent_backend_for_test("I can update the file.");
    route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"approve_workspace_writes_for_session"}"#,
    );

    let write = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","initialPrompt":"Update README","plannedTool":"filesystem.write","toolPath":"README.md"}"#,
    );
    assert_eq!(write["state"], "completed");

    let terminal = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","initialPrompt":"Run tests","plannedTool":"run_tests"}"#,
    );
    assert_eq!(terminal["state"], "blocked");
    assert_eq!(terminal["summary"], serde_json::Value::Null);
}

#[test]
fn native_iterative_loop_uses_the_mode_captured_by_its_session() {
    let workspace = TempDir::new().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(workspace.path())
        .status()
        .unwrap();
    assert!(status.success());
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace.path()),
    );
    let workspace_id = opened["workspaceId"].as_str().unwrap();
    route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"approve_workspace_writes_for_session"}"#,
    );
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"native.md","content":"mode bound\n"}}"#,
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"native.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Created native.md.","outcome":"changed","evidenceCallIds":["write-1","read-1"]}}"#,
    ]);

    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Create native.md"}}"#
        ),
    );

    assert_eq!(session["state"], "completed", "{session}");
    assert!(session["pendingApprovals"].as_array().unwrap().is_empty());
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("native.md")).unwrap(),
        "mode bound\n"
    );
}

#[test]
fn approval_mode_api_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_approval_modes.rs",
        include_str!("local_api_approval_modes.rs"),
        280,
    )
    .expect("approval mode api test should stay focused");
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.enable_test_controls_for_dev_server();
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

fn open_test_workspace(router: &mut LocalApiRouter) -> TempDir {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let root = fixture.path().join("desktoplab");
    std::fs::create_dir(&root).expect("workspace should be created");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    fixture
}

fn mode_ids(response: &Value) -> Vec<&str> {
    response["modes"]
        .as_array()
        .expect("modes should be an array")
        .iter()
        .map(|mode| mode["mode"].as_str().expect("mode id should be stable"))
        .collect()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
