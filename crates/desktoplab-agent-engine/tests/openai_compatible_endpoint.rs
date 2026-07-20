use desktoplab_agent_engine::{
    LocalInferenceAdapter, OpenAiCompatibleEndpoint, OpenAiCompatibleEndpointClass,
    OpenAiCompatibleEndpointError, OpenAiCompatibleEndpointPolicy,
};
use xtask::check_logical_line_limit;

#[test]
fn accepts_local_openai_compatible_endpoint_without_remote_policy() {
    let endpoint = OpenAiCompatibleEndpoint::validate(
        "http://127.0.0.1:1234/v1/chat/completions",
        OpenAiCompatibleEndpointPolicy::local_only(),
    )
    .expect("localhost endpoint should be accepted");

    assert_eq!(endpoint.class(), OpenAiCompatibleEndpointClass::Localhost);
    assert_eq!(endpoint.url(), "http://127.0.0.1:1234/v1/chat/completions");
}

#[test]
fn blocks_remote_endpoint_until_policy_allows_provider_egress() {
    let error = OpenAiCompatibleEndpoint::validate(
        "https://api.example.com/v1/chat/completions",
        OpenAiCompatibleEndpointPolicy::local_only(),
    )
    .expect_err("remote endpoint should require egress policy");

    assert_eq!(error, OpenAiCompatibleEndpointError::RemoteEndpointBlocked);
}

#[test]
fn accepts_remote_https_only_when_policy_allows_it() {
    let endpoint = OpenAiCompatibleEndpoint::validate(
        "https://api.example.com/v1/chat/completions",
        OpenAiCompatibleEndpointPolicy::allow_remote_https(),
    )
    .expect("remote https endpoint should be accepted with policy");

    assert_eq!(endpoint.class(), OpenAiCompatibleEndpointClass::RemoteHttps);
}

#[test]
fn rejects_secret_bearing_or_non_openai_compatible_urls() {
    assert_eq!(
        OpenAiCompatibleEndpoint::validate(
            "https://api.example.com/v1?api_key=sk-secret",
            OpenAiCompatibleEndpointPolicy::allow_remote_https(),
        )
        .unwrap_err(),
        OpenAiCompatibleEndpointError::SecretInUrl
    );
    assert_eq!(
        OpenAiCompatibleEndpoint::validate(
            "file:///tmp/socket",
            OpenAiCompatibleEndpointPolicy::allow_remote_https(),
        )
        .unwrap_err(),
        OpenAiCompatibleEndpointError::UnsupportedScheme
    );
}

#[test]
fn local_inference_adapter_can_only_attach_validated_endpoint() {
    let adapter = LocalInferenceAdapter::ollama("backend.custom", "runtime.custom", "model.custom")
        .try_with_openai_compatible_endpoint(
            "http://localhost:1234/v1/chat/completions",
            OpenAiCompatibleEndpointPolicy::local_only(),
        )
        .expect("validated endpoint should attach");

    assert_eq!(
        adapter,
        LocalInferenceAdapter::ollama("backend.custom", "runtime.custom", "model.custom")
            .with_openai_compatible_endpoint("http://localhost:1234/v1/chat/completions")
    );
}

#[test]
fn openai_compatible_endpoint_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/tests/openai_compatible_endpoint.rs",
        include_str!("openai_compatible_endpoint.rs"),
        140,
    )
    .expect("openai-compatible endpoint test should stay focused");
}
