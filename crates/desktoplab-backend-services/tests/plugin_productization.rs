use desktoplab_backend_services::{
    PluginContractHook, PluginDistributionKind, PluginExecutionBoundaryKind, PluginHookKind,
    PluginManifestLoader, PluginPermissionEngine, PluginPermissionKind, PluginProductizationHost,
    PluginRuntimeState, PluginTrustAction, PluginTrustState,
};
use xtask::check_logical_line_limit;

#[test]
fn manifest_loader_rejects_invalid_manifests_without_executing_plugin_code() {
    let loader = PluginManifestLoader::new("1.0.0");
    let invalid = r#"{"plugin_id":"","permissions":["tool.filesystem.write"]}"#;

    let result = loader.load_from_str(invalid);

    assert!(result.is_err());
    assert_eq!(loader.executed_plugin_code_count(), 0);
}

#[test]
fn unverified_manifest_loads_untrusted_with_explicit_permissions() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.gemini","contract_version":"1","trust":"unverified","permissions":["llm.chat"],"hooks":["provider"]}"#,
        )
        .unwrap();

    assert_eq!(manifest.plugin_id(), "plugin.gemini");
    assert_eq!(manifest.trust(), PluginTrustState::Unverified);
    assert_eq!(manifest.source(), "local_manifest");
    assert_eq!(manifest.install_policy(), "manual_review_required");
    assert_eq!(manifest.auth_policy(), "no_auth_requested");
    assert!(manifest.has_permission(PluginPermissionKind::LlmChat));
    assert!(!manifest.has_permission(PluginPermissionKind::FilesystemWrite));
}

#[test]
fn manifest_policy_fields_are_explicit_when_present() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.provider","contract_version":"1","source":"registry","trust":"unverified","install_policy":"signed_package_required","auth_policy":"oauth_device_flow","category":"provider","capabilities":["llm.chat"],"requires":["desktoplab.provider.v1"],"permissions":["llm.chat"],"hooks":["provider"]}"#,
        )
        .unwrap();

    assert_eq!(manifest.source(), "registry");
    assert_eq!(manifest.install_policy(), "signed_package_required");
    assert_eq!(manifest.auth_policy(), "oauth_device_flow");
    assert_eq!(manifest.category(), "provider");
    assert_eq!(manifest.capabilities(), &["llm.chat".to_string()]);
    assert_eq!(manifest.requires(), &["desktoplab.provider.v1".to_string()]);
    assert_eq!(manifest.trust(), PluginTrustState::Unverified);
}

#[test]
fn plugin_permission_engine_blocks_sensitive_tools_until_trust_is_explicitly_elevated() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.tools","contract_version":"1","trust":"unverified","permissions":["tool.filesystem.write"],"hooks":["tool"]}"#,
        )
        .unwrap();
    let mut host = PluginProductizationHost::new("1.0.0");
    host.load_manifest(manifest).unwrap();

    let denied = PluginPermissionEngine::default().authorize(&host, "plugin.tools");
    assert!(denied.is_blocked());
    assert!(
        denied
            .reasons()
            .contains(&"unverified_sensitive_permission".to_string())
    );

    assert_eq!(
        host.apply_trust_action("plugin.tools", PluginTrustAction::UserApproved)
            .unwrap_err(),
        "approval_record_required"
    );

    host.apply_trust_action_with_approval(
        "plugin.tools",
        PluginTrustAction::UserApproved,
        "approval.plugin.tools",
    )
    .unwrap();
    let approved = PluginPermissionEngine::default().authorize(&host, "plugin.tools");
    assert!(approved.is_allowed());
}

#[test]
fn blocked_plugins_cannot_load_and_unsupported_contract_versions_are_rejected() {
    let mut host = PluginProductizationHost::new("1.0.0");
    let loader = PluginManifestLoader::new("1.0.0");
    let blocked = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.blocked","contract_version":"1","trust":"verified","permissions":["llm.chat"],"hooks":["provider"],"state":"blocked"}"#,
        )
        .unwrap();
    assert_eq!(host.load_manifest(blocked).unwrap_err(), "plugin_blocked");

    let unsupported = loader.load_from_str(
        r#"{"plugin_id":"plugin.future","contract_version":"99","trust":"verified","permissions":["llm.chat"],"hooks":["provider"]}"#,
    );
    assert!(unsupported.is_err());
}

#[test]
fn plugin_hooks_depend_on_core_contracts_without_core_importing_implementations() {
    let hook = PluginContractHook::new(PluginHookKind::Runtime, "desktoplab.runtime.v1");

    assert_eq!(hook.contract_id(), "desktoplab.runtime.v1");
    assert!(hook.is_core_contract());
    assert!(!hook.imports_plugin_implementation());
}

#[test]
fn disabled_plugin_state_remains_visible_to_clients() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.disabled","contract_version":"1","trust":"verified","permissions":["llm.chat"],"hooks":["provider"],"state":"disabled"}"#,
        )
        .unwrap();

    assert_eq!(manifest.runtime_state(), PluginRuntimeState::Disabled);
}

#[test]
fn plugin_distribution_boundary_distinguishes_loaded_registry_and_marketplace() {
    let loader = PluginManifestLoader::new("1.0.0");
    let local = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.local","contract_version":"1","trust":"verified","permissions":["llm.chat"],"hooks":["provider"],"distribution":"local_loaded"}"#,
        )
        .unwrap();
    let registry = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.registry","contract_version":"1","trust":"unverified","permissions":["llm.chat"],"hooks":["provider"],"distribution":"registry_installable"}"#,
        )
        .unwrap();
    let marketplace = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.marketplace","contract_version":"1","trust":"unverified","permissions":["llm.chat"],"hooks":["provider"],"distribution":"marketplace_future"}"#,
        )
        .unwrap();

    assert_eq!(
        local.distribution_kind(),
        PluginDistributionKind::LocalLoaded
    );
    assert_eq!(
        registry.distribution_kind(),
        PluginDistributionKind::RegistryInstallable
    );
    assert_eq!(
        marketplace.distribution_kind(),
        PluginDistributionKind::MarketplaceFuture
    );
    assert!(!marketplace.install_boundary().available());
    assert_eq!(
        marketplace.install_boundary().reason(),
        "Plugin marketplace distribution is not available in this phase."
    );
}

#[test]
fn community_plugins_are_display_only_until_signed_out_of_process_runtime_exists() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.community","contract_version":"1","trust":"unverified","permissions":["llm.chat"],"hooks":["provider"],"distribution":"registry_installable"}"#,
        )
        .unwrap();

    assert_eq!(
        manifest.execution_boundary().kind(),
        PluginExecutionBoundaryKind::DisplayOnly
    );
    assert!(
        manifest
            .execution_boundary()
            .reason()
            .contains("signed package")
    );
    assert!(
        manifest
            .execution_boundary()
            .reason()
            .contains("out-of-process")
    );
}

#[test]
fn plugin_productization_sources_stay_below_line_count_guards() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backend-services/src/plugin_productization.rs",
            include_str!("../src/plugin_productization.rs"),
            180,
        ),
        (
            "crates/desktoplab-backend-services/src/plugin_productization/manifest.rs",
            include_str!("../src/plugin_productization/manifest.rs"),
            260,
        ),
        (
            "crates/desktoplab-backend-services/src/plugin_productization/permissions.rs",
            include_str!("../src/plugin_productization/permissions.rs"),
            240,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("plugin productization modules should stay focused");
    }
}
