use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn streaming_session_exposes_persistent_cancellable_job_state() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    open_test_workspace(&mut router, &fixture.path().join("desktoplab"));

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"work slowly","stream":true}"#,
    );

    assert_eq!(created["state"], "running");
    assert_eq!(created["job"]["state"], "running");
    assert_eq!(created["job"]["cancellable"], true);
    assert!(
        created["job"]["jobId"]
            .as_str()
            .unwrap()
            .starts_with("agent-job.")
    );
    assert!(created["job"]["startedAt"].as_str().unwrap().len() > 0);
    assert!(created["job"]["lastHeartbeatAt"].as_str().unwrap().len() > 0);
    assert_eq!(
        created["job"]["lastObservation"],
        "Waiting for model execution"
    );
    assert_eq!(created["controls"]["cancel"], true);

    let cancelled = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/control",
            created["sessionId"].as_str().unwrap()
        ),
        r#"{"action":"cancel"}"#,
    );
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(cancelled["job"]["state"], "cancelled");
    assert_eq!(
        cancelled["job"]["lastObservation"],
        "Waiting for model execution"
    );
    assert_eq!(cancelled["controls"]["cancel"], false);
}

#[test]
fn storage_restart_marks_running_agent_job_interrupted_with_recovery_guidance() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");

    let mut first_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should open storage");
    mark_setup_ready(&mut first_router);
    open_test_workspace(&mut first_router, &fixture.path().join("desktoplab"));
    let created = route_json(
        &mut first_router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"long run","stream":true}"#,
    );
    assert_eq!(created["job"]["state"], "running");

    let mut restarted_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should reopen storage");
    let workspace = route_json(&mut restarted_router, "GET", "/v1/agent/workspace", "");
    let session = &workspace["session"];

    assert_eq!(session["state"], "blocked");
    assert_eq!(session["blockedReason"], "long_running_job_interrupted");
    assert_eq!(session["job"]["state"], "interrupted");
    assert_eq!(
        session["job"]["lastObservation"],
        "Waiting for model execution"
    );
    assert!(
        timeline_text(session)
            .contains("Recover by reviewing partial evidence and starting a new prompt.")
    );
}

#[test]
fn local_api_agent_long_running_jobs_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_long_running_jobs.rs",
        include_str!("local_api_agent_long_running_jobs.rs"),
        140,
    )
    .expect("long-running agent job tests should stay focused");
}

fn timeline_text(session: &Value) -> String {
    session["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .map(|event| event["message"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
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
    let _ = route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    let _ = route_json(router, "POST", "/v1/setup/complete", "{}");
}

fn open_test_workspace(router: &mut LocalApiRouter, root: &std::path::Path) {
    std::fs::create_dir(root).expect("workspace should be created");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(root),
    );
}
