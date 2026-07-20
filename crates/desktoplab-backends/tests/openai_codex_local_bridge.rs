use desktoplab_backends::{
    BackendMessage, BackendToolSchema, OpenAiCodexCompletionPayload, OpenAiCodexPkceLogin,
    OpenAiCodexResponderCommandPayload, execute_openai_codex_responder_command,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use xtask::check_logical_line_limit;

#[test]
fn openai_codex_bridge_creates_pkce_login_from_vich_proven_endpoints() {
    let login = OpenAiCodexPkceLogin::from_verifier_source(
        b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "desktoplab_bridge_pair_abc123",
    )
    .expect("login metadata should be created");

    assert!(
        login
            .authorization_url()
            .starts_with("https://auth.openai.com/oauth/authorize?")
    );
    assert!(
        login
            .authorization_url()
            .contains("client_id=app_EMoamEEZ73f0CkXaXp7hrann")
    );
    assert!(
        login
            .authorization_url()
            .contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback")
    );
    assert!(login.authorization_url().contains("response_type=code"));
    assert!(
        login
            .authorization_url()
            .contains("scope=openid%20profile%20email%20offline_access")
    );
    assert!(
        login
            .authorization_url()
            .contains("state=desktoplab_bridge_pair_abc123")
    );
    assert!(
        login
            .authorization_url()
            .contains("code_challenge_method=S256")
    );
    assert_eq!(login.redirect_uri(), "http://localhost:1455/auth/callback");
    assert_eq!(login.token_url(), "https://auth.openai.com/oauth/token");
    assert_ne!(login.code_verifier(), login.code_challenge());
    assert!(login.code_verifier().len() >= 43);
}

#[test]
fn openai_codex_completion_payload_exposes_only_local_credential_references() {
    let payload = OpenAiCodexCompletionPayload::new(
        "desktoplab-macbook-pro",
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        "pairing-code-001",
        "desktoplab_bridge_pair_001",
        "Simone OpenAI Codex",
    )
    .expect("payload should be valid");

    let value = payload.to_json();
    assert_eq!(value["providerAccountLabel"], "Simone OpenAI Codex");
    assert_eq!(value["bridgeInstanceId"], "desktoplab-macbook-pro");
    assert_eq!(value["pairingId"], "desktoplab_bridge_pair_001");
    assert_eq!(
        value["localCredentialRef"],
        "vault://desktoplab/external-backend/openai-codex/profile/simone"
    );
    assert_eq!(value["bridgeCapabilities"][0], "chat.completions");
    assert!(!value.to_string().contains("access_token"));
    assert!(!value.to_string().contains("refresh_token"));
}

#[test]
fn openai_codex_responder_command_payload_stays_vault_ref_only() {
    let payload = OpenAiCodexResponderCommandPayload::for_agent_turn(
        vec![BackendMessage::user("Rispondi via Codex")],
        vec![BackendToolSchema::new(
            "desktoplab.complete",
            "Complete the current task.",
            json!({"type":"object"}),
        )],
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        "macos_keychain",
    )
    .expect("responder payload should be valid");

    let value = payload.to_json();
    assert!(value.get("credentialPath").is_none());
    assert!(
        value["prompt"]
            .as_str()
            .unwrap()
            .contains("desktoplab.complete")
    );
    assert_eq!(
        value["agentRequest"]["protocol"],
        "desktoplab.canonical-tools.v1"
    );
    assert_eq!(
        value["agentRequest"]["tools"][0]["function"]["name"],
        "desktoplab.complete"
    );
    assert_eq!(value["connection"]["tokenStorage"], "vault_ref_only");
    assert_eq!(
        value["connection"]["providerCredentialRef"],
        "vault://desktoplab/external-backend/openai-codex/profile/simone"
    );
    assert_eq!(value["connection"]["vaultKind"], "macos_keychain");
    assert!(!value.to_string().contains("access_token"));
    assert!(!value.to_string().contains("refresh_token"));
}

#[test]
fn openai_codex_responder_client_posts_vault_ref_payload_to_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback responder should bind");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("responder should receive request");
        let mut request = [0_u8; 4096];
        let read = stream
            .read(&mut request)
            .expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("POST / HTTP/1.1"));
        assert!(
            request.contains("vault://desktoplab/external-backend/openai-codex/profile/simone")
        );
        assert!(!request.contains("credentialPath"));
        assert!(!request.contains("access_token"));
        let body = r#"{"body":"Codex responder answered.","providerResponseId":"codex_resp_1"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should be written");
    });

    let payload = OpenAiCodexResponderCommandPayload::new(
        "Rispondi via Codex",
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        "macos_keychain",
    )
    .expect("payload should be valid");
    let output = execute_openai_codex_responder_command(&endpoint, &payload)
        .expect("loopback responder should answer");

    assert_eq!(output.body(), "Codex responder answered.");
    assert_eq!(output.provider_response_id(), "codex_resp_1");
    handle.join().expect("responder thread should finish");
}

#[test]
fn openai_codex_bridge_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/openai_codex_local_bridge.rs",
        include_str!("../src/openai_codex_local_bridge.rs"),
        320,
    )
    .expect("openai codex bridge source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/openai_codex_local_bridge.rs",
        include_str!("openai_codex_local_bridge.rs"),
        180,
    )
    .expect("openai codex bridge tests should stay focused");
}
