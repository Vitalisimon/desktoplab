use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn agent_workspace_route_summary_comes_from_execution_router() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(workspace["route"]["source"], "service_backed");
    assert_eq!(workspace["route"]["status"], "selected");
    assert_eq!(workspace["route"]["backendId"], "backend.ollama");
    assert_eq!(workspace["route"]["modelDisplayName"], "Gemma 4 12B Q4");
    assert_eq!(workspace["route"]["runtimeDisplayName"], "Ollama");
    assert!(workspace["route"]["reasons"].as_array().unwrap().is_empty());
}

#[test]
fn unavailable_local_model_blocks_route_with_plain_reason() {
    let mut router = LocalApiRouter::default();

    let route = route_json(
        &mut router,
        "GET",
        "/v1/routing/preference?localModelReady=false",
        "",
    );

    assert_eq!(route["source"], "service_backed");
    assert_eq!(route["status"], "blocked");
    assert_eq!(route["backendId"], Value::Null);
    assert!(
        route["blockedReasons"]
            .as_array()
            .unwrap()
            .contains(&Value::String("model not downloaded".to_string()))
    );
}

#[test]
fn route_preference_uses_backend_readiness_not_static_success() {
    let mut router = LocalApiRouter::default();

    let route = route_json(&mut router, "GET", "/v1/routing/preference", "");

    assert_eq!(route["source"], "service_backed");
    assert_eq!(route["status"], "blocked");
    assert_eq!(route["backendId"], Value::Null);
    assert_eq!(route["blockedReasons"][0], "runtime_and_model_not_verified");
}

#[test]
fn external_backends_show_configured_blocked_state_from_route_decision() {
    let mut router = LocalApiRouter::default();

    let backends = route_json(&mut router, "GET", "/v1/external-backends", "");

    assert_eq!(backends["source"], "service_backed");
    assert_eq!(backends["backends"][0]["backendId"], "backend.codex");
    assert_eq!(backends["backends"][0]["status"], "blocked");
    assert_eq!(
        backends["backends"][0]["routes"][0]["reason"],
        "credential missing"
    );
}

#[test]
fn execution_routes_expose_backend_capability_profiles() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);

    let local = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert_eq!(
        local["route"]["modelAgentCapability"]["class"],
        "agent_capable"
    );
    assert_eq!(
        local["route"]["modelAgentCapability"]["routeLabel"],
        "Local agent"
    );
    assert_eq!(
        local["route"]["modelAgentCapability"]["claim"],
        "The installed model fingerprint passed DesktopLab's tool protocol verification."
    );
    assert_contains(
        &local["route"]["backendCapabilities"],
        "agent.protocol.native_tool_calls",
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["nativeToolCalls"],
        true
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["protocolClass"],
        "native_tool"
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["canonicalExecutorPipeline"],
        true
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["fullCodingAgentEligible"],
        true
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["fallbackReason"],
        Value::Null
    );
    assert_eq!(
        local["route"]["backendToolCalling"]["endpoint"],
        "http://127.0.0.1:11434/api/chat"
    );
    assert_not_contains(
        &local["route"]["backendCapabilities"],
        "agent.protocol.strict_json_actions",
    );

    let external = route_json(&mut router, "GET", "/v1/external-backends", "");
    assert_contains(
        &external["backends"][0]["capabilities"],
        "external.egress.requires_approval",
    );
    assert_not_contains(
        &external["backends"][0]["capabilities"],
        "tools.filesystem.write.approval",
    );
}

#[test]
fn local_api_execution_routes_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_execution_routes.rs",
        include_str!("local_api_execution_routes.rs"),
        220,
    )
    .expect("execution route test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/execution_routes.rs",
        include_str!("../src/execution_routes.rs"),
        400,
    )
    .expect("execution route source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/execution_external_routes.rs",
        include_str!("../src/execution_external_routes.rs"),
        180,
    )
    .expect("external execution route source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/execution_tool_calling.rs",
        include_str!("../src/execution_tool_calling.rs"),
        120,
    )
    .expect("execution tool calling source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/execution_tool_calling_evidence.rs",
        include_str!("../src/execution_tool_calling_evidence.rs"),
        90,
    )
    .expect("execution tool calling evidence should stay focused");
}

fn assert_contains(value: &Value, expected: &str) {
    assert!(
        value
            .as_array()
            .unwrap()
            .contains(&Value::String(expected.to_string())),
        "{value:?} should contain {expected}"
    );
}

fn assert_not_contains(value: &Value, unexpected: &str) {
    assert!(
        !value
            .as_array()
            .unwrap()
            .contains(&Value::String(unexpected.to_string())),
        "{value:?} should not contain {unexpected}"
    );
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
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}

fn open_test_workspace(router: &mut LocalApiRouter) -> TempDir {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let root = fixture.path().join("desktoplab");
    std::fs::create_dir(&root).expect("workspace should be created");
    post(
        router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    fixture
}
