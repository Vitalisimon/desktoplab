use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn native_iterative_approval_resumes_after_router_restart() {
    let db = NamedTempFile::new().expect("temp sqlite file");
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    mark_setup_ready(&mut router);
    let workspace_id = open_workspace(&mut router, workspace.path())["workspaceId"]
        .as_str()
        .unwrap()
        .to_string();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"restart.md","content":"durable loop\n"}}"#,
    ]);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Create restart.md."}}"#
        ),
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    restarted.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"restart.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Created restart.md after restart.","outcome":"changed","evidenceCallIds":["write-1","read-1"]}}"#,
    ]);
    route_json(
        &mut restarted,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("restart.md")).unwrap(),
        "durable loop\n"
    );
}

#[test]
fn completed_model_plan_survives_router_restart() {
    let db = NamedTempFile::new().expect("temp sqlite file");
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
    mark_setup_ready(&mut router);
    let workspace_id = open_workspace(&mut router, workspace.path())["workspaceId"]
        .as_str()
        .unwrap()
        .to_string();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"plan-1","tool":"desktoplab.update_plan","arguments":{"steps":[{"step":"Inspect","status":"completed"},{"step":"Verify","status":"in_progress"}]}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Plan saved.","outcome":"executed","evidenceCallIds":["plan-1"]}}"#,
    ]);
    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"plan work"}}"#
        ),
    );
    let session_id = created["sessionId"].as_str().unwrap().to_string();
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    let sessions = route_json(
        &mut restarted,
        "GET",
        &format!("/v1/sessions?workspace_id={workspace_id}"),
        "",
    );
    let restored = sessions["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|session| session["sessionId"] == session_id)
        .unwrap();

    assert_eq!(
        restored["plan"],
        "[completed] Inspect\n[in_progress] Verify"
    );
}

#[test]
fn native_iterative_restart_test_stays_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-control-plane/tests/local_api_native_iterative_restart.rs",
            include_str!("local_api_native_iterative_restart.rs"),
            210,
        ),
        (
            "crates/desktoplab-control-plane/src/router/persistence_iterative.rs",
            include_str!("../src/router/persistence_iterative.rs"),
            100,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit)
            .expect("native iterative persistence source should stay focused");
    }
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
    assert!(output.status.success());
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
