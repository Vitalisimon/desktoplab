use desktoplab_control_plane::LocalApiRouter;
use desktoplab_policy::{
    Action, DecisionOutcome, EgressAccountMode, EgressClassification, PolicyEngine,
    ProviderEgressContext, ProviderEgressPolicy,
};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent, ToolOutcome};
use xtask::check_logical_line_limit;

#[test]
fn provider_egress_requires_policy_and_blocks_local_only_data() {
    let default = PolicyEngine::default_conservative().evaluate(Action::ProviderEgressWithAccount(
        ProviderEgressContext::new(
            EgressClassification::SafeToEgress,
            EgressAccountMode::ApiKeyBilling,
        ),
    ));
    let local_only = PolicyEngine::default_conservative()
        .with_provider_egress(ProviderEgressPolicy::Allow)
        .evaluate(Action::ProviderEgressWithAccount(
            ProviderEgressContext::new(
                EgressClassification::LocalOnly,
                EgressAccountMode::ApiKeyBilling,
            ),
        ));

    assert_eq!(default.outcome(), DecisionOutcome::RequiresApproval);
    assert_eq!(local_only.outcome(), DecisionOutcome::Denied);
}

#[test]
fn protected_workspace_paths_are_not_available_for_provider_context() {
    for path in [".git/config", ".env", ".ssh/id_rsa", "credentials/token"] {
        let outcome = ToolGateway::new(PolicyEngine::default_conservative())
            .authorize(ToolIntent::filesystem_read(path));

        assert_eq!(outcome, ToolOutcome::Blocked("local_only_path".to_string()));
    }
}

#[test]
fn provider_routes_redact_secrets_and_require_approval_before_egress() {
    let mut router = LocalApiRouter::default();
    router.use_fake_openai_codex_native_vault_for_test();

    let providers = route_json(&mut router, "GET", "/v1/providers", "");
    assert_eq!(providers["providers"][0]["egress"], "requires_approval");

    let connected = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai/connect",
        r#"{"accountMode":"api_key_billing","apiKey":"sk-provider-secret","operatingSystem":"macos"}"#,
    );
    let endpoint = route_json(
        &mut router,
        "POST",
        "/v1/providers/provider.openai-compatible/connect",
        r#"{"accountMode":"custom_endpoint","endpointUrl":"https://api.example.com/v1?api_key=sk-url-secret","allowRemoteHttps":true}"#,
    );

    assert_payload_redacted(&connected);
    assert_payload_redacted(&endpoint);
    assert_eq!(endpoint["blockedReason"], "secretinurl");
}

#[test]
fn provider_security_policy_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/provider_security_policy.rs",
        include_str!("provider_security_policy.rs"),
        140,
    )
    .expect("provider security policy test should stay focused");
}

fn assert_payload_redacted(value: &serde_json::Value) {
    let payload = value.to_string();
    assert!(!payload.contains("sk-provider-secret"));
    assert!(!payload.contains("sk-url-secret"));
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
