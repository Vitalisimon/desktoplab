mod support;

use desktoplab_backends::{
    BackendModelCapabilities, ModelProtocolCertificationState, OllamaToolProtocolCanary,
};
use support::{ProviderMock, ProviderMockConfig};

const SUCCESS_BODY: &str = r#"{"message":{"content":"","tool_calls":[{"function":{"name":"desktoplab.list_files","arguments":{"path":"."}}}]}}"#;

#[test]
fn successful_canary_is_cached_by_capability_fingerprint() {
    let mock = ollama_mock(SUCCESS_BODY);
    let capabilities = capabilities("digest-a");
    let canary = OllamaToolProtocolCanary::default();

    let first = canary.certify(mock.endpoint(), &capabilities, 5);
    let second = canary.certify(mock.endpoint(), &capabilities, 5);

    assert_eq!(first.state(), ModelProtocolCertificationState::Certified);
    assert_eq!(second, first);
    let evidence = mock.finish();
    let request_body = evidence.request["body"].as_str().unwrap();
    assert!(request_body.contains("desktoplab.list_files"));
    assert!(request_body.contains(r#""think":false"#));
    assert!(request_body.contains(r#""num_predict":128"#));
    assert_eq!(evidence.driver, "mock");
    assert!(evidence.jsonl.contains(r#""path":"/api/chat""#));
}

#[test]
fn changed_digest_runs_a_new_canary() {
    let first_mock = ollama_mock(SUCCESS_BODY);
    let second_mock = ollama_mock(SUCCESS_BODY);
    let canary = OllamaToolProtocolCanary::default();

    let first = canary.certify(first_mock.endpoint(), &capabilities("digest-a"), 5);
    let second = canary.certify(second_mock.endpoint(), &capabilities("digest-b"), 5);

    assert_eq!(first.state(), ModelProtocolCertificationState::Certified);
    assert_eq!(second.state(), ModelProtocolCertificationState::Certified);
    assert_ne!(first, second);
    first_mock.finish();
    second_mock.finish();
}

#[test]
fn canary_fails_closed_on_noncanonical_tool_output() {
    let mock = ollama_mock(
        r#"{"message":{"content":"done","tool_calls":[{"function":{"name":"write_file","arguments":{"path":"."}}}]}}"#,
    );

    let result = OllamaToolProtocolCanary::default().certify(
        mock.endpoint(),
        &capabilities("digest-failed"),
        5,
    );

    assert_eq!(result.state(), ModelProtocolCertificationState::Failed);
    assert_eq!(
        result.failure_reason(),
        Some("ollama_canary_contract_mismatch")
    );
    mock.finish();
}

#[test]
fn exact_content_tool_call_certifies_constrained_json_protocol() {
    let mock = ollama_mock(
        r#"{"message":{"content":"{\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"}}"#,
    );

    let result = OllamaToolProtocolCanary::default().certify(
        mock.endpoint(),
        &capabilities("digest-constrained"),
        5,
    );

    assert_eq!(result.state(), ModelProtocolCertificationState::Certified);
    assert_eq!(
        result.protocol(),
        Some(desktoplab_backends::ModelToolProtocolKind::ConstrainedJson)
    );
    mock.finish();
}

#[test]
fn protocol_canary_sources_stay_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-backends/src/model_protocol_certification.rs",
            include_str!("../src/model_protocol_certification.rs"),
            120,
        ),
        (
            "crates/desktoplab-backends/src/ollama_protocol_canary.rs",
            include_str!("../src/ollama_protocol_canary.rs"),
            190,
        ),
        (
            "crates/desktoplab-backends/tests/ollama_protocol_canary.rs",
            include_str!("ollama_protocol_canary.rs"),
            130,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit).unwrap();
    }
}

fn capabilities(digest: &str) -> BackendModelCapabilities {
    BackendModelCapabilities::reported(
        "backend.ollama",
        "model.test",
        Some(digest.to_string()),
        Some(32_768),
        ["completion", "tools"],
    )
}

fn ollama_mock(body: &'static str) -> ProviderMock {
    ProviderMock::start(ProviderMockConfig {
        expected_path: "/api/chat",
        required_authorization: None,
        status: 200,
        body,
    })
}
