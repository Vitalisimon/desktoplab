use desktoplab_backends::{
    OpenAiCodexDeviceAuthorizationPollRequest, OpenAiCodexDeviceTokenExchangeRequest,
    OpenAiCodexResponderCommandOutput,
};
use xtask::check_logical_line_limit;

#[test]
fn openai_codex_device_authorization_poll_and_exchange_match_vich_bridge() {
    let poll = OpenAiCodexDeviceAuthorizationPollRequest::new("device_auth_123", "ABCD-EFGH")
        .expect("poll request should be valid")
        .to_json();
    assert_eq!(
        poll["url"],
        "https://auth.openai.com/api/accounts/deviceauth/token"
    );
    assert_eq!(poll["method"], "POST");
    assert_eq!(poll["body"]["device_auth_id"], "device_auth_123");
    assert_eq!(poll["body"]["user_code"], "ABCD-EFGH");

    let exchange =
        OpenAiCodexDeviceTokenExchangeRequest::new("auth_code_from_poll", "verifier_from_poll")
            .expect("exchange request should be valid")
            .to_json();
    assert_eq!(exchange["url"], "https://auth.openai.com/oauth/token");
    assert_eq!(exchange["method"], "POST");
    assert_eq!(
        exchange["body"]["client_id"],
        "app_EMoamEEZ73f0CkXaXp7hrann"
    );
    assert_eq!(exchange["body"]["code"], "auth_code_from_poll");
    assert_eq!(exchange["body"]["code_verifier"], "verifier_from_poll");
    assert_eq!(exchange["body"]["grant_type"], "authorization_code");
    assert_eq!(
        exchange["body"]["redirect_uri"],
        "https://auth.openai.com/deviceauth/callback"
    );
}

#[test]
fn openai_codex_responder_output_parses_or_derives_response_id() {
    let explicit = OpenAiCodexResponderCommandOutput::parse(
        r#"{"body":"Collegamento OpenAI verificato.","providerResponseId":"openai_codex_response_001"}"#,
    )
    .expect("explicit responder output should parse");
    assert_eq!(explicit.body(), "Collegamento OpenAI verificato.");
    assert_eq!(explicit.provider_response_id(), "openai_codex_response_001");

    let derived =
        OpenAiCodexResponderCommandOutput::parse(r#"{"body":"Collegamento OpenAI verificato."}"#)
            .expect("responder output should derive missing response id");
    assert_eq!(derived.provider_response_id().len(), 46);
    assert!(
        derived
            .provider_response_id()
            .starts_with("openai_codex_response_")
    );
}

#[test]
fn openai_codex_device_auth_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/openai_codex_device_auth.rs",
        include_str!("../src/openai_codex_device_auth.rs"),
        220,
    )
    .expect("openai codex device auth source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backends/src/openai_codex_device_http.rs",
        include_str!("../src/openai_codex_device_http.rs"),
        180,
    )
    .expect("openai codex device http source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/openai_codex_device_auth.rs",
        include_str!("openai_codex_device_auth.rs"),
        120,
    )
    .expect("openai codex device auth tests should stay focused");
}
