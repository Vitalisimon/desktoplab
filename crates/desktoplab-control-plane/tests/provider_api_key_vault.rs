use desktoplab_control_plane::LocalApiRouter;
use xtask::check_logical_line_limit;

#[test]
fn api_key_provider_flow_uses_vault_reference_without_plaintext() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();

    let connected = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"api_key_billing","apiKey":"sk-live-secret","operatingSystem":"macos"}"#,
    );
    assert_eq!(connected["status"], "connected");
    assert_eq!(connected["vaultKind"], expected_native_vault_kind());
    assert_eq!(connected["plaintextFallbackAllowed"], false);
    assert_payload_redacted(&connected);

    let tested = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/test",
        r#"{"accountMode":"api_key_billing"}"#,
    );
    assert_eq!(tested["state"], "degraded");
    assert_payload_redacted(&tested);

    let removed = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/disconnect",
        r#"{"accountMode":"api_key_billing"}"#,
    );
    assert_eq!(removed["status"], "removed");
    assert_payload_redacted(&removed);
}

fn expected_native_vault_kind() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos_keychain"
    } else if cfg!(target_os = "windows") {
        "windows_credential_manager"
    } else {
        "linux_secret_service"
    }
}

#[test]
fn api_key_provider_flow_uses_host_platform_instead_of_client_platform_claim() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"api_key_billing","apiKey":"sk-live-secret","operatingSystem":"plan9"}"#,
    );

    assert_eq!(blocked["status"], "connected");
    assert_eq!(blocked["plaintextFallbackAllowed"], false);
    assert_eq!(
        blocked["diagnostic"]["redactedEvidence"],
        "credential=[REDACTED]; token_storage=vault_ref_only"
    );
    assert_payload_redacted(&blocked);
}

#[test]
fn provider_api_key_vault_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/provider_api_key_vault.rs",
        include_str!("provider_api_key_vault.rs"),
        130,
    )
    .expect("provider api key vault test should stay focused");
}

fn assert_payload_redacted(value: &serde_json::Value) {
    let payload = value.to_string();
    assert!(!payload.contains("sk-live-secret"));
    assert!(!payload.contains("sk-test-secret"));
}

fn route_json(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> serde_json::Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
