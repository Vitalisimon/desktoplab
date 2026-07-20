use desktoplab_acp_plugin::{AcpBackendPlugin, AcpPluginLoader, PluginTrust};
use desktoplab_agent_session::{SessionOwner, SessionState};
use desktoplab_backend_services::{
    PluginManifestLoader, PluginPermissionEngine, PluginProductizationHost, PluginTrustAction,
};
use desktoplab_backends::{
    ClaudeAgentSdkBridge, ClaudeBridgeConfig, CodexAppServerBridge, CodexBridgeConfig,
    ExternalEvent,
};

#[test]
fn plugin_and_external_backend_bridge_e2e_preserves_desktoplab_ownership() {
    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.codex","contract_version":"1","trust":"unverified","permissions":["llm.chat"],"hooks":["backend"]}"#,
        )
        .unwrap();
    let mut host = PluginProductizationHost::new("1.0.0");
    host.load_manifest(manifest).unwrap();
    assert!(
        PluginPermissionEngine::default()
            .authorize(&host, "plugin.codex")
            .is_allowed()
    );

    host.apply_trust_action_with_approval(
        "plugin.codex",
        PluginTrustAction::UserApproved,
        "approval.plugin.codex",
    )
    .unwrap();
    let codex = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));
    let imported = codex.import_events(
        "session.bridge.e2e",
        vec![
            ExternalEvent::text_delta("plan"),
            ExternalEvent::Completed("complete".into()),
        ],
    );

    assert_eq!(imported.session().owner(), SessionOwner::DesktopLab);
    assert_eq!(imported.session().state(), SessionState::Completed);
}

#[test]
fn acp_and_claude_bridge_paths_are_plugin_or_external_backend_owned_by_desktoplab() {
    let acp = AcpPluginLoader::default().load(AcpBackendPlugin::new_unverified("plugin.acp"));
    assert_eq!(acp.trust(), PluginTrust::Unverified);
    assert!(!acp.is_core_component());

    let claude = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let blocked = claude.record_failure(
        "session.claude.e2e",
        desktoplab_backends::BridgeCallFailure::new("network", "temporary outage"),
    );

    assert_eq!(blocked.session().owner(), SessionOwner::DesktopLab);
    assert_eq!(blocked.session().state(), SessionState::Blocked);
}
