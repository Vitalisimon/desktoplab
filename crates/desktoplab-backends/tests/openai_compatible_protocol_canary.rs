use desktoplab_backends::{
    BackendModelCapabilities, ModelProtocolCertificationState, ModelToolProtocolKind,
    OpenAiCompatibleToolProtocolCanary,
};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn native_tool_response_certifies_the_exact_capability_fingerprint() {
    let capabilities = capabilities("backend.lm-studio", "desktoplab-gpt-oss-20b");
    let canary = OpenAiCompatibleToolProtocolCanary::with_response_for_test(json!({
        "choices":[{"message":{"content":"","tool_calls":[{
            "id":"call-1","type":"function","function":{
                "name":"desktoplab.list_files","arguments":"{\"path\":\".\"}"
            }
        }]}}]
    }));

    let certification = canary.certify(
        "http://127.0.0.1:12345",
        &capabilities,
        ModelToolProtocolKind::NativeTools,
        10,
    );

    assert_eq!(
        certification.state(),
        ModelProtocolCertificationState::Certified
    );
    assert_eq!(
        certification.protocol(),
        Some(ModelToolProtocolKind::NativeTools)
    );
    assert!(certification.is_certified_for(capabilities.fingerprint()));
}

#[test]
fn constrained_json_response_certifies_mlx_without_claiming_native_tools() {
    let capabilities = capabilities("backend.mlx-lm", "mlx-community/SmolLM3-3B-4bit");
    let canary = OpenAiCompatibleToolProtocolCanary::with_response_for_test(json!({
        "choices":[{"message":{"content":"{\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"}}]
    }));

    let certification = canary.certify(
        "http://127.0.0.1:18080",
        &capabilities,
        ModelToolProtocolKind::ConstrainedJson,
        10,
    );

    assert_eq!(
        certification.state(),
        ModelProtocolCertificationState::Certified
    );
    assert_eq!(
        certification.protocol(),
        Some(ModelToolProtocolKind::ConstrainedJson)
    );
}

#[test]
fn protocol_mismatch_and_non_loopback_endpoint_fail_closed() {
    let capabilities = capabilities("backend.lm-studio", "desktoplab-gpt-oss-20b");
    let constrained = OpenAiCompatibleToolProtocolCanary::with_response_for_test(json!({
        "choices":[{"message":{"content":"{\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"}}]
    }));
    let mismatch = constrained.certify(
        "http://127.0.0.1:12345",
        &capabilities,
        ModelToolProtocolKind::NativeTools,
        10,
    );
    let remote = constrained.certify_fresh(
        "https://models.example.com",
        &capabilities,
        ModelToolProtocolKind::ConstrainedJson,
        10,
    );
    let disguised_remote = constrained.certify_fresh(
        "http://localhost:1234@models.example.com",
        &capabilities,
        ModelToolProtocolKind::ConstrainedJson,
        10,
    );

    assert_eq!(mismatch.state(), ModelProtocolCertificationState::Failed);
    assert_eq!(remote.state(), ModelProtocolCertificationState::Failed);
    assert_eq!(
        disguised_remote.failure_reason(),
        Some("local_canary_endpoint_not_loopback")
    );
    assert_eq!(
        remote.failure_reason(),
        Some("local_canary_endpoint_not_loopback")
    );
}

#[test]
fn openai_compatible_canary_source_stays_focused() {
    assert!(
        include_str!("../src/openai_compatible_protocol_canary.rs").contains(r#""max_tokens":512"#),
        "the live canary must leave enough completion budget for reasoning-capable local models"
    );
    check_logical_line_limit(
        "crates/desktoplab-backends/src/openai_compatible_protocol_canary.rs",
        include_str!("../src/openai_compatible_protocol_canary.rs"),
        260,
    )
    .expect("OpenAI-compatible protocol canary should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/openai_compatible_protocol_canary.rs",
        include_str!("openai_compatible_protocol_canary.rs"),
        130,
    )
    .expect("OpenAI-compatible protocol canary tests should stay focused");
}

fn capabilities(backend: &str, model: &str) -> BackendModelCapabilities {
    BackendModelCapabilities::reported(
        backend,
        model,
        Some("runtime@loopback".to_string()),
        Some(32_768),
        ["completion", "tools"],
    )
}
