use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn provider_list_exposes_non_secret_auth_profile_health() {
    let mut router = LocalApiRouter::default();

    let providers = route_json(&mut router, "GET", "/v1/providers", "");
    let health = &providers["providers"][0]["authProfileHealth"];

    assert_eq!(health["authMode"], "api_key_billing");
    assert_eq!(health["credentialReferenceKind"], "none");
    assert_eq!(health["lastHealthState"], "missing_credential");
    assert_eq!(
        health["fallbackApproval"],
        "explicit_user_approval_required"
    );
    assert!(!providers.to_string().contains("sk-live"));
    assert!(!providers.to_string().contains("access_token"));
}

#[test]
fn subscription_fallback_to_api_key_requires_explicit_approval() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();
    let pairing = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account"}"#,
    );
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "authorization_code_test",
        "code_verifier_test",
    );
    post(
        &mut router,
        "/v1/provider-bridges/openai-codex/pairing/poll",
        &format!(
            r#"{{"pairingId":"{}"}}"#,
            pairing["pairingId"].as_str().unwrap()
        ),
    );

    let diagnostics = route_json(
        &mut router,
        "GET",
        "/v1/providers/provider.openai/diagnostics",
        "",
    );
    let health = &diagnostics["authProfileHealth"];

    assert_eq!(health["authMode"], "subscription_account");
    assert_eq!(health["credentialReferenceKind"], "vault_ref");
    assert_eq!(health["lastHealthState"], "probe_required");
    assert_eq!(health["cooldownState"], "not_probed");
    assert_eq!(diagnostics["state"], "degraded");
    assert_eq!(health["fallbackOrder"][0], "subscription_account");
    assert_eq!(health["fallbackOrder"][1], "api_key_billing");
    assert_eq!(
        health["fallbackApproval"],
        "explicit_user_approval_required"
    );
    assert!(!diagnostics.to_string().contains("authorization_code_test"));
}

#[test]
fn provider_auth_health_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_provider_auth_health.rs",
        include_str!("local_api_provider_auth_health.rs"),
        180,
    )
    .expect("provider auth health tests should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}
