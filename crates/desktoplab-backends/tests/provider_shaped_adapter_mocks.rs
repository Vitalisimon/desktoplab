mod support;

use desktoplab_backends::{
    BackendModelCapabilities, BackendModelInventory, BackendPrompt, BackendToolSchema,
    LocalEndpoint, OllamaExecutionBackend, OpenAiCompatibleLocalExecutionBackend,
};
use serde_json::json;
use support::{ProviderMock, ProviderMockConfig};

#[test]
fn real_openai_compatible_adapter_reaches_provider_shaped_mock() {
    let mock = ProviderMock::start(ProviderMockConfig {
        expected_path: "/v1/chat/completions",
        required_authorization: None,
        status: 200,
        body: r#"{"choices":[{"message":{"content":"","tool_calls":[{"id":"call.1","type":"function","function":{"name":"desktoplab.read_file","arguments":"{\"path\":\"README.md\"}"}}]}}]}"#,
    });
    let backend = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.mock-openai",
        LocalEndpoint::available(mock.endpoint()),
        BackendModelInventory::available(&["model.mock"]),
    );
    let prompt = BackendPrompt::new("model.mock", "Read the repository").with_tools(vec![
        BackendToolSchema::new(
            "desktoplab.read_file",
            "Read one workspace file.",
            json!({"type":"object","properties":{"path":{"type":"string"}}}),
        ),
    ]);

    let output: serde_json::Value =
        serde_json::from_str(&backend.execute_chat(&prompt).unwrap()).unwrap();
    let evidence = mock.finish();
    assert_eq!(output["tool"], "desktoplab.read_file");
    assert_eq!(evidence.request["path"], "/v1/chat/completions");
    assert!(
        evidence.request["body"]
            .as_str()
            .unwrap()
            .contains("model.mock")
    );
    assert_eq!(evidence.driver, "mock");
    assert!(evidence.jsonl.contains(r#""driver":"mock""#));
}

#[test]
fn real_adapters_fail_closed_on_auth_rate_limit_and_malformed_payloads() {
    let auth = ProviderMock::start(ProviderMockConfig {
        expected_path: "/v1/chat/completions",
        required_authorization: Some("Bearer sk-test-secret"),
        status: 200,
        body: r#"{"choices":[]}"#,
    });
    let openai = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.mock-auth",
        LocalEndpoint::available(auth.endpoint()),
        BackendModelInventory::available(&["model.mock"]),
    );
    assert!(
        openai
            .execute_chat(&BackendPrompt::new("model.mock", "hello"))
            .unwrap_err()
            .contains("401")
    );
    let auth_evidence = auth.finish();
    assert!(!auth_evidence.jsonl.contains("sk-test-secret"));

    let rate_limit = ProviderMock::start(ProviderMockConfig {
        expected_path: "/api/chat",
        required_authorization: None,
        status: 429,
        body: r#"{"error":"rate_limited"}"#,
    });
    let ollama = OllamaExecutionBackend::new(BackendModelInventory::available(&["model.mock"]))
        .with_model_capabilities([BackendModelCapabilities::reported(
            "backend.ollama",
            "model.mock",
            Some("digest-mock".to_string()),
            Some(8_192),
            ["tools"],
        )]);
    assert!(
        ollama
            .execute_chat(
                rate_limit.endpoint(),
                &BackendPrompt::new("model.mock", "hello")
            )
            .unwrap_err()
            .contains("429")
    );
    assert_eq!(rate_limit.finish().request["path"], "/api/chat");

    let malformed = ProviderMock::start(ProviderMockConfig {
        expected_path: "/v1/chat/completions",
        required_authorization: None,
        status: 200,
        body: "not-json",
    });
    let openai = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.mock-malformed",
        LocalEndpoint::available(malformed.endpoint()),
        BackendModelInventory::available(&["model.mock"]),
    );
    assert!(
        openai
            .execute_chat(&BackendPrompt::new("model.mock", "hello"))
            .unwrap_err()
            .contains("response_json")
    );
    malformed.finish();
}

#[test]
fn provider_mock_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backends/tests/support/provider_mock.rs",
        include_str!("support/provider_mock.rs"),
        190,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-backends/tests/provider_shaped_adapter_mocks.rs",
        include_str!("provider_shaped_adapter_mocks.rs"),
        170,
    )
    .unwrap();
}
