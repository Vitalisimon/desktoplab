use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn provider_connect_stores_reads_and_never_returns_the_api_key() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();

    let connected = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"api_key_billing","apiKey":"sk-test-secret","operatingSystem":"macos"}"#,
    );

    assert_eq!(connected["source"], "service_backed");
    assert_eq!(connected["status"], "connected");
    assert_eq!(connected["accountMode"], "api_key_billing");
    assert_eq!(
        connected["vaultRef"],
        "vault://desktoplab/provider/provider.openai:api_key_billing"
    );
    let vault_ref = connected["vaultRef"].as_str().expect("vault ref");
    assert_eq!(
        router.openai_codex_native_secret_for_test(vault_ref),
        Some("sk-test-secret".to_string())
    );
    let checked = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/test",
        r#"{"accountMode":"api_key_billing"}"#,
    );
    assert_eq!(checked["state"], "degraded");
    assert_eq!(
        checked["redactedEvidence"],
        "credential=[REDACTED]; vault_read=verified; remote_call=not_run"
    );
    assert!(!connected.to_string().contains("sk-test-secret"));
    assert!(!checked.to_string().contains("sk-test-secret"));
}

#[test]
fn missing_api_key_blocks_even_when_the_client_spoofs_an_operating_system() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"api_key_billing","operatingSystem":"plan9"}"#,
    );

    assert_eq!(blocked["source"], "service_backed");
    assert_eq!(blocked["status"], "blocked");
    assert_eq!(blocked["diagnostic"]["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "credential_missing");
    assert_eq!(blocked["vaultRef"], Value::Null);
}

#[test]
fn non_api_key_account_modes_are_blocked_until_bridge_is_certified() {
    let mut router = LocalApiRouter::default();

    let subscription = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"subscription_account","operatingSystem":"macos"}"#,
    );

    assert_eq!(subscription["status"], "blocked");
    assert_eq!(subscription["accountMode"], "subscription_account");
    assert_eq!(subscription["vaultRef"], serde_json::Value::Null);
    assert_eq!(
        subscription["blockedReason"],
        "account_bridge_not_certified"
    );
}

#[test]
fn custom_endpoint_validates_url_but_blocks_until_health_check_is_certified() {
    let mut router = LocalApiRouter::default();

    let endpoint = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai-compatible/connect",
        r#"{"accountMode":"custom_endpoint","endpointUrl":"http://127.0.0.1:1234/v1/chat/completions"}"#,
    );

    assert_eq!(endpoint["status"], "blocked");
    assert_eq!(endpoint["accountMode"], "custom_endpoint");
    assert_eq!(
        endpoint["blockedReason"],
        "custom_endpoint_health_check_not_certified"
    );
    assert!(!endpoint.to_string().contains("sk-"));
}

#[test]
fn stale_persisted_provider_reference_is_not_reported_as_connected_after_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    {
        let mut router = LocalApiRouter::with_storage_path(&db_path).expect("router should open");
        router.use_fake_openai_codex_native_vault_for_test();
        route_json(
            &mut router,
            "POST",
            "/v1/providers/provider.openai/connect",
            r#"{"accountMode":"api_key_billing","apiKey":"sk-test-secret","operatingSystem":"macos"}"#,
        );
    }

    let mut restarted = LocalApiRouter::with_storage_path(&db_path).expect("router should restart");
    let providers = route_json(&mut restarted, "GET", "/v1/providers", "");
    let diagnostics = route_json(
        &mut restarted,
        "GET",
        "/v1/providers/provider.openai/diagnostics",
        "",
    );

    assert_eq!(providers["providers"][0]["status"], "missing_credential");
    assert_eq!(
        providers["providers"][0]["activeAccountMode"],
        "api_key_billing"
    );
    assert_eq!(providers["providers"][0]["vaultRef"], Value::Null);
    assert_eq!(diagnostics["state"], "missing_credential");
    assert!(!providers.to_string().contains("sk-test-secret"));
}

#[test]
fn provider_disconnect_persists_removed_metadata_across_router_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    {
        let mut router = LocalApiRouter::with_storage_path(&db_path).expect("router should open");
        router.use_fake_openai_codex_native_vault_for_test();
        route_json(
            &mut router,
            "POST",
            "/v1/providers/provider.openai/connect",
            r#"{"accountMode":"api_key_billing","apiKey":"sk-test-secret","operatingSystem":"macos"}"#,
        );
        route_json(
            &mut router,
            "POST",
            "/v1/providers/provider.openai/disconnect",
            r#"{"accountMode":"api_key_billing"}"#,
        );
    }

    let mut restarted = LocalApiRouter::with_storage_path(&db_path).expect("router should restart");
    let providers = route_json(&mut restarted, "GET", "/v1/providers", "");

    assert_eq!(providers["providers"][0]["status"], "missing_credential");
    assert_eq!(
        providers["providers"][0]["diagnostic"]["state"],
        "missing_credential"
    );
    assert_eq!(providers["providers"][0]["vaultRef"], Value::Null);
}

#[test]
fn local_api_provider_vault_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_provider_vault.rs",
        include_str!("local_api_provider_vault.rs"),
        230,
    )
    .expect("provider vault route test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/provider_routes.rs",
        include_str!("../src/provider_routes.rs"),
        240,
    )
    .expect("provider route source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/provider_routes/account_profile.rs",
        include_str!("../src/provider_routes/account_profile.rs"),
        100,
    )
    .expect("provider account profile source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/provider_routes/credentials.rs",
        include_str!("../src/provider_routes/credentials.rs"),
        230,
    )
    .expect("provider credential coordinator should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
