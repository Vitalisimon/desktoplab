use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn session_start_uses_backend_owned_response_for_workbench_timeline() {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );
    router.complete_agent_backend_for_test("Use tests first, then make a focused change.");

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect repo and propose first test"}"#,
    );

    assert_eq!(created["sessionId"], "session.1");
    assert_eq!(created["state"], "completed");
    assert_eq!(created["plan"], "Inspect repo and propose first test");
    assert_eq!(created["timeline"][0]["kind"], "planning");
    assert_eq!(
        created["timeline"][0]["message"],
        "Inspect repo and propose first test"
    );
    assert_eq!(created["timeline"][1]["kind"], "assistant");
    assert_ne!(
        created["timeline"][1]["message"], "Terminal command",
        "timeline must not come from canned payloads"
    );
    assert_eq!(
        created["timeline"][1]["message"],
        "Use tests first, then make a focused change."
    );
}

#[test]
fn local_only_provider_tool_request_is_recovered_without_secret_disclosure() {
    let fixture = TempDir::new().unwrap();
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    std::fs::write(workspace_root.join(".env"), "SECRET=fixture\n").unwrap();
    let git = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&workspace_root)
        .output()
        .unwrap();
    assert!(git.status.success());
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"I will inspect the requested file.","tool":"desktoplab.read_file","arguments":{"path":".env"}}"#,
    );
    let workspace_id =
        route_json(&mut router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &serde_json::json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Read .env before coding"
        })
        .to_string(),
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(completed["timeline"][1]["kind"], "assistant");
    assert_eq!(
        completed["timeline"][1]["message"],
        "I will inspect the requested file."
    );
    let timeline = completed["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| {
            event["kind"].as_str() == Some("tool_decision")
                && event["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("state=failed"))
        }),
        "{completed}"
    );
    assert!(
        completed
            .to_string()
            .contains("executor_reason=local_only_path")
    );
    assert!(!completed.to_string().contains("SECRET=fixture"));
}

#[test]
fn local_api_agent_session_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_session.rs",
        include_str!("local_api_agent_session.rs"),
        150,
    )
    .expect("agent session route test should stay focused");
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
