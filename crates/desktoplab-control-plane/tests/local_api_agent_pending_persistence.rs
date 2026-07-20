use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn pending_agent_file_action_survives_router_restart() {
    let db = NamedTempFile::new().expect("temp sqlite file");
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    mark_setup_ready(&mut router);
    let workspace_record = open_workspace(&mut router, workspace.path());
    let workspace_id = workspace_record["workspaceId"].as_str().unwrap();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo persisted.md.","desktoplabAction":{"kind":"create_file","path":"persisted.md","content":"# Persisted\n"}}"##,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"crea persisted.md"}}"#
        ),
    );
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    restarted.complete_agent_backend_for_test("Persisted action completed with executor evidence.");
    route_json(
        &mut restarted,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(
        &mut restarted,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"continue","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("persisted.md")).unwrap(),
        "# Persisted\n"
    );
}

#[test]
fn consumed_agent_approval_does_not_reappear_after_router_restart() {
    let db = NamedTempFile::new().expect("temp sqlite file");
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    mark_setup_ready(&mut router);
    let workspace_record = open_workspace(&mut router, workspace.path());
    let workspace_id = workspace_record["workspaceId"].as_str().unwrap();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo done.md.","desktoplabAction":{"kind":"create_file","path":"done.md","content":"# Done\n"}}"##,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"crea done.md"}}"#
        ),
    );
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"continue","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed");
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    let approvals = route_json(&mut restarted, "GET", "/v1/approvals", "");
    let records = approvals["approvals"].as_array().unwrap();

    assert!(records.iter().all(|approval| {
        approval["state"].as_str() != Some("pending") && approval["consumed"] == true
    }));
    let workbench = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert_eq!(workbench["session"]["sessionId"], blocked["sessionId"]);
    assert_eq!(workbench["session"]["state"], "completed");
}

#[test]
fn agent_pending_persistence_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_pending_persistence.rs",
        include_str!("local_api_agent_pending_persistence.rs"),
        185,
    )
    .expect("agent pending persistence test should stay focused");
}

fn open_workspace(router: &mut LocalApiRouter, path: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    )
}

fn create_repo(path: &std::path::Path) {
    let output = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .expect("git init should run");
    assert!(output.status.success(), "git init failed");
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
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
