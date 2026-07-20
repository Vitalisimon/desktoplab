use desktoplab_backends::ModelToolProtocolKind;
use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn installed_model_without_capability_probe_is_exposed_as_chat_only() {
    let temp = tempdir().expect("workspace should exist");
    let mut router = ready_ollama_router();
    open_test_workspace(&mut router, temp.path());

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let route = &workspace["route"];

    assert_eq!(route["modelAgentCapability"]["class"], "chat_capable");
    assert_eq!(route["backendToolCalling"]["nativeToolCalls"], false);
    assert_eq!(
        route["backendToolCalling"]["fallbackReason"],
        "model_tool_capability_unverified"
    );
    assert_eq!(
        route["backendToolCalling"]["toolCapabilityState"],
        "probe_required"
    );
    assert!(!contains(
        &route["backendCapabilities"],
        "agent.protocol.native_tool_calls"
    ));
}

#[test]
fn runtime_reported_tools_enable_the_agent_route_for_that_model() {
    let temp = tempdir().expect("workspace should exist");
    let mut router = ready_ollama_router();
    open_test_workspace(&mut router, temp.path());
    router.mark_ollama_model_capabilities_for_test(
        "gemma4:12b",
        &["completion", "tools", "thinking"],
    );

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let route = &workspace["route"];

    assert_eq!(route["modelAgentCapability"]["class"], "agent_capable");
    assert_eq!(route["backendToolCalling"]["nativeToolCalls"], true);
    assert_eq!(
        route["backendToolCalling"]["toolCapabilityState"],
        "confirmed"
    );
    assert_eq!(
        route["backendToolCalling"]["toolProtocolCertification"],
        "certified"
    );
    assert!(
        route["backendToolCalling"]["capabilityFingerprint"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(contains(
        &route["backendCapabilities"],
        "agent.protocol.native_tool_calls"
    ));
}

#[test]
fn certified_constrained_json_is_agent_capable_without_native_tool_claims() {
    let temp = tempdir().expect("workspace should exist");
    let mut router = ready_ollama_router();
    open_test_workspace(&mut router, temp.path());
    router.mark_ollama_model_capabilities_with_protocol_for_test(
        "gemma4:12b",
        &["completion", "tools"],
        ModelToolProtocolKind::ConstrainedJson,
    );

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let route = &workspace["route"];

    assert_eq!(
        route["backendToolCalling"]["protocolClass"],
        "constrained_json"
    );
    assert_eq!(route["backendToolCalling"]["nativeToolCalls"], false);
    assert_eq!(route["backendToolCalling"]["fullCodingAgentEligible"], true);
    assert!(route["backendToolCalling"]["fallbackReason"].is_null());
    assert!(contains(
        &route["backendCapabilities"],
        "agent.protocol.constrained_json"
    ));
    assert!(!contains(
        &route["backendCapabilities"],
        "agent.protocol.native_tool_calls"
    ));
}

#[test]
fn capability_fingerprint_survives_control_plane_restart() {
    let temp = tempdir().unwrap();
    let database = temp.path().join("desktoplab.db");
    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database)
        .expect("router should open");
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama ready");
    router.mark_model_verified_without_capabilities_for_test(
        "runtime.ollama",
        "model.gemma4-12b-q4",
        "gemma4:12b installed",
    );
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    drop(router);

    let mut reopened =
        LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database).unwrap();
    let state = route_json(&mut reopened, "GET", "/v1/app/state", "");

    assert_eq!(
        state["readiness"]["evidence"]["modelCapabilities"]["modelId"],
        "gemma4:12b"
    );
    assert!(
        state["readiness"]["evidence"]["modelCapabilities"]["fingerprint"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(
        state["readiness"]["evidence"]["modelCapabilities"]["toolProtocolCertification"]["state"],
        "certified"
    );
}

fn ready_ollama_router() -> LocalApiRouter {
    let mut router = LocalApiRouter::default();
    router.set_local_model_inventory_for_test(&["gemma4:12b"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama ready");
    router.mark_model_verified_without_capabilities_for_test(
        "runtime.ollama",
        "model.gemma4-12b-q4",
        "gemma4:12b installed",
    );
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    router
}

fn open_test_workspace(router: &mut LocalApiRouter, root: &std::path::Path) {
    let workspace = root.join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should be created");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace),
    );
}

fn contains(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .unwrap()
        .contains(&Value::String(expected.to_string()))
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
