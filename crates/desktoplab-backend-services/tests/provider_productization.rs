use desktoplab_backend_services::{
    ProviderConnectivityInput, ProviderConnectivityState, ProviderEndpointClass,
    ProviderManifestTrust, ProviderPluginManifest, ProviderProductizationCatalog,
    ProviderReadinessStatus, ProviderRoutePlanner, ProviderRoutePreference,
};
use desktoplab_vault::{AuthModeMetadata, FakeVault, SecretRef, SecretScope};
use xtask::check_logical_line_limit;

#[test]
fn cloud_provider_catalog_keeps_capabilities_in_data() {
    let catalog = ProviderProductizationCatalog::default_cloud();

    let openai = catalog
        .provider("provider.openai")
        .expect("OpenAI provider exists");
    let anthropic = catalog
        .provider("provider.anthropic")
        .expect("Anthropic provider exists");

    assert!(openai.capabilities().contains(&"llm.chat".to_string()));
    assert!(
        openai
            .capabilities()
            .contains(&"tools.function_call".to_string())
    );
    assert!(anthropic.capabilities().contains(&"llm.chat".to_string()));
    assert_eq!(
        catalog.readiness("provider.openai", None).status(),
        ProviderReadinessStatus::MissingCredential
    );
}

#[test]
fn provider_readiness_exposes_supported_account_modes() {
    let catalog = ProviderProductizationCatalog::default_cloud();
    let openai = catalog
        .readiness("provider.openai", None)
        .supported_account_modes()
        .to_vec();
    let anthropic = catalog
        .readiness("provider.anthropic", None)
        .supported_account_modes()
        .to_vec();
    let custom = catalog
        .readiness("provider.openai-compatible", None)
        .supported_account_modes()
        .to_vec();

    assert_eq!(
        openai,
        vec![
            AuthModeMetadata::ApiKeyBilling,
            AuthModeMetadata::SubscriptionAccount,
            AuthModeMetadata::OauthDevice,
        ]
    );
    assert_eq!(
        anthropic,
        vec![
            AuthModeMetadata::ApiKeyBilling,
            AuthModeMetadata::SubscriptionAccount,
        ]
    );
    assert_eq!(custom, vec![AuthModeMetadata::CustomEndpoint]);
}

#[test]
fn openai_compatible_endpoint_validation_classifies_localhost() {
    let catalog = ProviderProductizationCatalog::default_cloud();

    let local = catalog
        .validate_openai_compatible_endpoint("http://127.0.0.1:1234/v1")
        .expect("localhost endpoint should be accepted");
    let cloud = catalog
        .validate_openai_compatible_endpoint("https://api.example.com/v1")
        .expect("https endpoint should be accepted");

    assert_eq!(local.class(), ProviderEndpointClass::Localhost);
    assert_eq!(cloud.class(), ProviderEndpointClass::Remote);
    assert!(
        catalog
            .validate_openai_compatible_endpoint("file:///tmp/key")
            .is_err()
    );
}

#[test]
fn provider_plugin_manifests_are_bounded_by_known_capabilities() {
    let gemini = ProviderPluginManifest::new(
        "provider.gemini",
        ProviderManifestTrust::Unverified,
        &["llm.chat"],
    );
    let invalid = ProviderPluginManifest::new(
        "provider.bad",
        ProviderManifestTrust::Unverified,
        &["filesystem.raw_access"],
    );

    assert!(gemini.validate().is_ok());
    assert!(gemini.route_sensitive_work().is_err());
    assert!(invalid.validate().is_err());
}

#[test]
fn connectivity_diagnostics_redact_credentials_and_do_not_include_workspace_content() {
    let catalog = ProviderProductizationCatalog::default_cloud();
    let secret_ref = SecretRef::new(SecretScope::Provider, "provider.openai:api-key");
    let diagnostic = catalog.connectivity_diagnostic(ProviderConnectivityInput::new(
        "provider.openai",
        Some(secret_ref),
        "sk-live-secret",
    ));

    assert_eq!(diagnostic.state(), ProviderConnectivityState::Ready);
    assert_eq!(diagnostic.redacted_authorization(), "Bearer [REDACTED]");
    assert!(!diagnostic.diagnostic_payload().contains("sk-live-secret"));
    assert!(!diagnostic.diagnostic_payload().contains("src/main.rs"));
}

#[test]
fn provider_route_selection_prefers_local_and_marks_cloud_approval() {
    let planner = ProviderRoutePlanner::new(ProviderRoutePreference::LocalFirst);
    let catalog = ProviderProductizationCatalog::default_cloud();

    let local = catalog.local_provider_candidate("backend.ollama", &["llm.chat"]);
    let cloud = catalog.cloud_provider_candidate("provider.openai", &["llm.chat"], "low");

    let local_decision = planner.select(&["llm.chat"], vec![local.clone(), cloud.clone()]);
    let cloud_only = planner.select(&["llm.chat"], vec![cloud]);

    assert_eq!(local_decision.selected_id(), Some("backend.ollama"));
    assert!(!local_decision.requires_provider_egress_approval());
    assert_eq!(cloud_only.selected_id(), Some("provider.openai"));
    assert!(cloud_only.requires_provider_egress_approval());
    assert_eq!(cloud_only.cost_hint(), Some("metadata:low".to_string()));
}

#[test]
fn provider_catalog_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/provider_productization.rs",
        include_str!("../src/provider_productization.rs"),
        320,
    )
    .expect("provider productization source should stay focused");
}

#[test]
fn provider_vault_reference_is_never_read_without_explicit_adapter_call() {
    let vault = FakeVault::default();
    let missing_ref = SecretRef::new(SecretScope::Provider, "provider.openai:api-key");
    let catalog = ProviderProductizationCatalog::default_cloud();

    assert_eq!(
        catalog
            .readiness("provider.openai", Some(missing_ref))
            .status(),
        ProviderReadinessStatus::CredentialReferenceMissing
    );
    drop(vault);
}
