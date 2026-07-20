use desktoplab_agent_session::SessionOwner;
use desktoplab_backends::{
    BridgeFailureCode, BridgeReadinessProbe, BridgeReadinessService, BridgeStatus,
    ClaudeAgentSdkBridge, ClaudeBridgeConfig, CodexAppServerBridge, CodexBridgeConfig,
};
use desktoplab_domain::{AccountMode, ProviderId};
use xtask::check_logical_line_limit;

#[test]
fn codex_readiness_is_distinct_from_provider_identity() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::with_provider(
        "http://127.0.0.1:1455",
        ProviderId::new("provider.openai"),
    ));

    let readiness = BridgeReadinessService::check_codex(
        &bridge,
        BridgeReadinessProbe::available("codex app server reachable"),
    );

    assert_eq!(readiness.status(), BridgeStatus::Ready);
    assert_eq!(readiness.backend_id(), "backend.codex-app-server");
    assert_eq!(readiness.provider_id(), Some("provider.openai"));
    assert_eq!(readiness.auth_mode(), AccountMode::SubscriptionAccount);
    assert!(readiness.provider_identity_is_metadata_only());
}

#[test]
fn claude_failures_normalize_to_backend_failure_codes() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));

    let readiness =
        BridgeReadinessService::check_claude(&bridge, BridgeReadinessProbe::failed("sdk missing"));

    assert_eq!(readiness.status(), BridgeStatus::Blocked);
    assert_eq!(readiness.auth_mode(), AccountMode::SubscriptionAccount);
    assert_eq!(
        readiness.failure_code(),
        Some(BridgeFailureCode::SdkUnavailable)
    );
}

#[test]
fn acp_plugin_boundary_remains_optional() {
    let readiness = BridgeReadinessService::check_acp_plugin(None);

    assert_eq!(readiness.status(), BridgeStatus::OptionalUnavailable);
    assert_eq!(
        readiness.failure_code(),
        Some(BridgeFailureCode::PluginMissing)
    );
}

#[test]
fn external_backends_cannot_own_session_state() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));

    let session = bridge.create_session("session.external");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert!(bridge.requires_desktoplab_policy());
}

#[test]
fn bridge_readiness_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/bridge_readiness.rs",
        include_str!("../src/bridge_readiness.rs"),
        260,
    )
    .expect("bridge readiness source should stay below the line-count guard");
}
