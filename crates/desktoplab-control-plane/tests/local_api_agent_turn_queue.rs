use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};
use xtask::check_logical_line_limit;

#[test]
fn concurrent_prompts_queue_fifo_and_resume_after_restart() {
    let db = NamedTempFile::new().unwrap();
    let workspace = TempDir::new().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(workspace.path())
            .status()
            .unwrap()
            .success()
    );
    let mut router = LocalApiRouter::with_storage_path(db.path()).unwrap();
    mark_setup_ready(&mut router);
    let opened = post(
        &mut router,
        "/v1/workspaces/open",
        &serde_json::json!({"path":workspace.path()}).to_string(),
    );
    let workspace_id = opened["workspaceId"].as_str().unwrap();
    let running = post(
        &mut router,
        "/v1/sessions",
        &serde_json::json!({"workspaceId":workspace_id,"executionBackendId":"backend.ollama","initialPrompt":"slow","stream":true}).to_string(),
    );
    let session_id = running["sessionId"].as_str().unwrap();
    let path = format!("/v1/sessions/{session_id}/messages");
    let first = post(
        &mut router,
        &path,
        &serde_json::json!({"workspaceId":workspace_id,"executionBackendId":"backend.ollama","prompt":"first queued"}).to_string(),
    );
    let second = post(
        &mut router,
        &path,
        &serde_json::json!({"workspaceId":workspace_id,"executionBackendId":"backend.ollama","prompt":"second queued"}).to_string(),
    );
    assert_eq!(first["queuedTurns"][0]["prompt"], "first queued");
    assert_eq!(second["queuedTurns"][1]["prompt"], "second queued");
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).unwrap();
    restarted.complete_agent_backend_for_test("first completed");
    let recovered = get(&mut restarted, "/v1/agent/workspace");
    assert_eq!(recovered["session"]["state"], "blocked");
    assert_eq!(recovered["session"]["queuedTurns"][0]["state"], "queued");
    let resumed = post(
        &mut restarted,
        &format!("/v1/sessions/{session_id}/control"),
        r#"{"action":"resume"}"#,
    );
    assert_eq!(resumed["sessionId"], session_id);
    assert_eq!(resumed["state"], "completed");
    assert_eq!(resumed["queuedTurns"][0]["prompt"], "second queued");
    assert_eq!(
        get(&mut restarted, "/v1/sessions")["sessions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn running_turn_rejects_context_it_cannot_preserve_in_the_queue() {
    let workspace = TempDir::new().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(workspace.path())
            .status()
            .unwrap()
            .success()
    );
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let opened = post(
        &mut router,
        "/v1/workspaces/open",
        &serde_json::json!({"path":workspace.path()}).to_string(),
    );
    let workspace_id = opened["workspaceId"].as_str().unwrap();
    let running = post(
        &mut router,
        "/v1/sessions",
        &serde_json::json!({"workspaceId":workspace_id,"executionBackendId":"backend.ollama","initialPrompt":"slow","stream":true}).to_string(),
    );
    let path = format!(
        "/v1/sessions/{}/messages",
        running["sessionId"].as_str().unwrap()
    );
    let response = router.route("POST", &path, &serde_json::json!({
        "workspaceId":workspace_id,"executionBackendId":"backend.ollama","prompt":"use this",
        "externalAttachments":[{"name":"brief.txt","size":5,"mediaType":"text/plain","contentText":"notes","contentSha256":"sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309"}]
    }).to_string()).unwrap();
    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("QUEUED_TURN_CONTEXT_UNSUPPORTED"));
}

#[test]
fn running_turn_cannot_be_queued_from_another_workspace() {
    let first_workspace = TempDir::new().unwrap();
    let second_workspace = TempDir::new().unwrap();
    for workspace in [&first_workspace, &second_workspace] {
        assert!(
            std::process::Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(workspace.path())
                .status()
                .unwrap()
                .success()
        );
    }
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let first = post(
        &mut router,
        "/v1/workspaces/open",
        &serde_json::json!({"path":first_workspace.path()}).to_string(),
    );
    let first_workspace_id = first["workspaceId"].as_str().unwrap();
    let running = post(
        &mut router,
        "/v1/sessions",
        &serde_json::json!({"workspaceId":first_workspace_id,"executionBackendId":"backend.ollama","initialPrompt":"slow","stream":true}).to_string(),
    );
    let session_id = running["sessionId"].as_str().unwrap();
    let second = post(
        &mut router,
        "/v1/workspaces/open",
        &serde_json::json!({"path":second_workspace.path()}).to_string(),
    );
    let response = router
        .route(
            "POST",
            &format!("/v1/sessions/{session_id}/messages"),
            &serde_json::json!({
                "workspaceId":second["workspaceId"],
                "executionBackendId":"backend.ollama",
                "prompt":"must not cross repositories"
            })
            .to_string(),
        )
        .unwrap();

    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("WORKSPACE_SESSION_MISMATCH"));
}

#[test]
fn turn_queue_test_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_turn_queue.rs",
        include_str!("local_api_agent_turn_queue.rs"),
        220,
    )
    .unwrap();
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.6.2");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) -> Value {
    route(router, "POST", path, body)
}
fn get(router: &mut LocalApiRouter, path: &str) -> Value {
    route(router, "GET", path, "")
}
fn route(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
