mod support;

use desktoplab_backends::{
    BackendModelCapabilities, BackendModelInventory, BackendPrompt, BackendToolSchema,
    ModelToolProtocolCertification, ModelToolProtocolKind, OllamaExecutionBackend,
};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use serde_json::json;
use support::{ProviderMock, ProviderMockConfig};

#[test]
fn certified_constrained_json_executes_through_the_canonical_adapter_shape() {
    let mock = ProviderMock::start(ProviderMockConfig {
        expected_path: "/api/chat",
        required_authorization: None,
        status: 200,
        body: r#"{"message":{"content":"{\"name\":\"desktoplab.read_file\",\"arguments\":{\"path\":\"README.md\"}}"}}"#,
    });
    let profile = constrained_profile("model.test");
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model.test"]))
        .with_model_capabilities([profile]);
    let prompt =
        BackendPrompt::new("model.test", "Read README").with_tools(vec![BackendToolSchema::new(
            "desktoplab.read_file",
            "Read a workspace file.",
            json!({
                "type":"object",
                "properties":{"path":{"type":"string"}},
                "required":["path"]
            }),
        )]);

    let output = backend.execute_chat(mock.endpoint(), &prompt).unwrap();
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(output["tool"], "desktoplab.read_file");
    assert_eq!(output["arguments"]["path"], "README.md");
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["agent.protocol.constrained_json"]),
        vec![backend.route_candidate_for_model("model.test")],
    );
    assert_eq!(route.status(), RouteStatus::Selected);
    let evidence = mock.finish();
    assert_eq!(evidence.request["path"], "/api/chat");
    assert_eq!(evidence.driver, "mock");
    assert!(evidence.jsonl.contains(r#""path":"/api/chat""#));
    let request_body: serde_json::Value =
        serde_json::from_str(evidence.request["body"].as_str().unwrap()).unwrap();
    assert_eq!(request_body["format"], "json");
}

#[test]
fn constrained_json_certification_accepts_a_later_native_tool_call() {
    let mock = ProviderMock::start(ProviderMockConfig {
        expected_path: "/api/chat",
        required_authorization: None,
        status: 200,
        body: r#"{"message":{"content":"","tool_calls":[{"function":{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}}]}}"#,
    });
    let profile = constrained_profile("model.test");
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model.test"]))
        .with_model_capabilities([profile]);
    let prompt =
        BackendPrompt::new("model.test", "Read README").with_tools(vec![BackendToolSchema::new(
            "desktoplab.read_file",
            "Read a workspace file.",
            json!({
                "type":"object",
                "properties":{"path":{"type":"string"}},
                "required":["path"]
            }),
        )]);

    let output = backend.execute_chat(mock.endpoint(), &prompt).unwrap();
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(output["tool"], "desktoplab.read_file");
    assert_eq!(output["arguments"]["path"], "README.md");
    mock.finish();
}

#[test]
fn constrained_json_rejects_prose_wrapped_or_non_object_calls() {
    for body in [
        "Use this {\"name\":\"desktoplab.read_file\",\"arguments\":{}}",
        "{\"name\":\"desktoplab.read_file\",\"arguments\":[]}",
    ] {
        assert!(desktoplab_backends::parse_constrained_tool_text(body).is_err());
    }
}

fn constrained_profile(model: &str) -> BackendModelCapabilities {
    let profile = BackendModelCapabilities::reported(
        "backend.ollama",
        model,
        Some("digest-constrained".to_string()),
        Some(32_768),
        ["completion", "tools"],
    );
    let certification = ModelToolProtocolCertification::certified_as(
        profile.fingerprint(),
        ModelToolProtocolKind::ConstrainedJson,
    );
    profile.with_tool_protocol_certification(certification)
}
