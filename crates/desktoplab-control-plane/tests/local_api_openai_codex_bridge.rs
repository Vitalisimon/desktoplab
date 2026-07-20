use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::net::TcpListener;
use xtask::check_logical_line_limit;

#[test]
fn openai_codex_pairing_start_returns_authorization_metadata_without_tokens() {
    let mut router = LocalApiRouter::default();
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );

    let started = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account","stateSeed":"desktoplab-test"}"#,
    );

    assert_eq!(started["status"], "authorization_required");
    assert_eq!(started["providerId"], "provider.openai");
    assert_eq!(started["accountMode"], "subscription_account");
    assert_eq!(started["tokenStorage"], "vault_ref_only");
    assert_eq!(
        started["authorizationUrl"],
        "https://auth.openai.com/codex/device"
    );
    assert_eq!(started["deviceLogin"]["userCode"], "ABCD-EFGH");
    assert_eq!(started["deviceLogin"]["deviceAuthId"], "device_auth_test");
    assert!(!started.to_string().contains("access_token"));
    assert!(!started.to_string().contains("refresh_token"));
}

#[test]
fn openai_codex_pairing_completion_persists_subscription_bridge_account() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );
    let started = start_pairing(&mut router);
    let vault_ref = "vault://desktoplab/external-backend/openai-codex/profile/simone";
    router.store_openai_codex_native_secret_for_test(vault_ref, "test-codex-secret");
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let responder_url = format!(
        "http://{}",
        listener.local_addr().expect("listener address")
    );
    let body = completion_body(&started, &responder_url);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/complete",
        &body,
    );
    let providers = route_json(&mut router, "GET", "/v1/providers", "");

    assert_eq!(completed["status"], "connected");
    assert_eq!(
        completed["vaultRef"],
        "vault://desktoplab/external-backend/openai-codex/profile/simone"
    );
    assert_eq!(completed["bridgeResponderUrl"], responder_url);
    assert_eq!(providers["providers"][0]["status"], "connected");
    assert_eq!(
        providers["providers"][0]["activeAccountMode"],
        "subscription_account"
    );
    assert_eq!(
        providers["providers"][0]["vaultRef"],
        "vault://desktoplab/external-backend/openai-codex/profile/simone"
    );
}

#[test]
fn openai_codex_device_poll_stores_token_in_native_vault() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    router.use_fake_openai_codex_native_vault_for_test();
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );
    let started = start_pairing(&mut router);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/poll",
        &format!(
            r#"{{"pairingId":"{}"}}"#,
            started["pairingId"].as_str().unwrap()
        ),
    );
    let providers = route_json(&mut router, "GET", "/v1/providers", "");
    let routes = route_json(&mut router, "GET", "/v1/routing/options", "");

    assert_eq!(completed["status"], "connected");
    assert_eq!(completed["vaultKind"], "macos_keychain");
    assert!(
        completed["vaultRef"]
            .as_str()
            .unwrap()
            .starts_with("vault://desktoplab/external-backend/openai-codex/")
    );
    assert!(completed["bridgeResponderUrl"].is_null());
    assert_eq!(providers["providers"][0]["status"], "connected");
    assert_eq!(routes["options"][2]["status"], "unavailable");
    assert!(!completed.to_string().contains("access_token"));
    assert!(!temp.path().join("openai-codex").exists());
    assert!(
        router
            .openai_codex_native_secret_for_test(completed["vaultRef"].as_str().unwrap())
            .unwrap()
            .contains("refresh_token")
    );
}

#[test]
fn openai_codex_pairing_completion_rejects_remote_responders_and_raw_tokens() {
    let mut router = LocalApiRouter::default();
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );
    let started = start_pairing(&mut router);
    let remote_body = completion_body(&started, "https://api.example.com/bridge");
    let token_body = completion_body(&started, "http://127.0.0.1:43109")
        .replace('}', r#","access_token":"secret"}"#);

    let remote = route_response(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/complete",
        &remote_body,
    );
    let token = route_response(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/complete",
        &token_body,
    );

    assert_eq!(remote.status(), "400 Bad Request");
    assert!(remote.body().contains("loopback"));
    assert_eq!(token.status(), "400 Bad Request");
    assert!(token.body().contains("Raw provider tokens"));
}

#[test]
fn openai_codex_bridge_route_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_openai_codex_bridge.rs",
        include_str!("local_api_openai_codex_bridge.rs"),
        210,
    )
    .expect("openai codex bridge route tests should stay focused");
}

fn start_pairing(router: &mut LocalApiRouter) -> Value {
    route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account","stateSeed":"desktoplab-test"}"#,
    )
}

fn completion_body(started: &Value, responder_url: &str) -> String {
    format!(
        r#"{{
            "pairingId":"{}",
            "pairingCode":"{}",
            "bridgeInstanceId":"desktoplab-macbook",
            "providerAccountLabel":"Simone OpenAI Codex",
            "localCredentialRef":"vault://desktoplab/external-backend/openai-codex/profile/simone",
            "responderUrl":"{}"
        }}"#,
        started["pairingId"].as_str().unwrap(),
        started["pairingCode"].as_str().unwrap(),
        responder_url
    )
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = route_response(router, method, path, body);
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn route_response(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> desktoplab_control_plane::ApiRouteResponse {
    router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"))
}
