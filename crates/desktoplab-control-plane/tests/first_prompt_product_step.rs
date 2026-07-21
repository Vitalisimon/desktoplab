use desktoplab_control_plane::LocalApiRouter;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn first_prompt_creates_backend_owned_session_step_and_events() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);
    router.complete_agent_backend_for_test("Posso ispezionare il repository, proporre modifiche, eseguire test e riassumere i risultati.");

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Map the repository before editing"}"#,
    );

    assert_eq!(created["sessionId"], "session.1");
    assert_eq!(created["state"], "completed");
    assert_eq!(created["timeline"][0]["kind"], "planning");
    assert_eq!(
        created["timeline"][0]["message"],
        "Map the repository before editing"
    );
    assert_eq!(created["timeline"][1]["kind"], "assistant");
    assert_eq!(
        created["timeline"][1]["message"],
        "Posso ispezionare il repository, proporre modifiche, eseguire test e riassumere i risultati."
    );
    assert_eq!(created["summary"], "agent loop completed");

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    assert!(replay["frames"].as_array().unwrap().iter().any(|frame| {
        frame["payload"]
            .as_str()
            .unwrap()
            .contains("agent.step.completed")
    }));
}

#[test]
fn first_prompt_records_runtime_failure_without_claiming_local_inference_is_unconfigured() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);
    router.fail_agent_backend_for_test();

    let failed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Read protected files","toolPath":".env"}"#,
    );

    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["timeline"][1]["kind"], "failed");
    assert_eq!(failed["timeline"][1]["message"], "local_inference_failed");
    assert_eq!(
        failed["failureClassification"]["primary"],
        "local_inference_failure"
    );
    assert_eq!(
        failed["failureClassification"]["userMessage"],
        "Local inference failed before the agent could continue."
    );
    assert!(
        failed["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["kind"].as_str() != Some("tool_decision")),
        "tool policy must not run before inference readiness exists"
    );

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    assert!(replay["frames"].as_array().unwrap().iter().any(|frame| {
        frame["payload"]
            .as_str()
            .unwrap()
            .contains("agent.step.failed")
    }));
}

#[test]
fn first_prompt_product_step_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/first_prompt_product_step.rs",
        include_str!("first_prompt_product_step.rs"),
        140,
    )
    .expect("first prompt product step test should stay focused");
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

fn route_json(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> serde_json::Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
