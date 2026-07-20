use desktoplab_agent_session::SessionOwner;
use desktoplab_backends::{CodexAppServerBridge, CodexBridgeConfig};
use desktoplab_domain::{AccountMode, ExecutionBackendKind, ProviderId};
use xtask::check_logical_line_limit;

#[test]
fn codex_bridge_remains_an_execution_backend() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));

    assert_eq!(bridge.backend_id(), "backend.codex-app-server");
    assert_eq!(bridge.backend_kind(), ExecutionBackendKind::ExternalAgent);
}

#[test]
fn desktoplab_owns_codex_bridge_session_and_policy_boundary() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));
    let session = bridge.create_session("session.codex");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.codex-app-server");
    assert!(bridge.requires_desktoplab_policy());
}

#[test]
fn provider_identity_remains_separate_when_codex_credentials_are_needed() {
    let config = CodexBridgeConfig::with_provider_auth_mode(
        "https://codex.example.internal",
        ProviderId::new("provider.openai"),
        AccountMode::ApiKeyBilling,
    );
    let bridge = CodexAppServerBridge::new(config);

    assert_eq!(
        bridge.provider_id(),
        Some(&ProviderId::new("provider.openai"))
    );
    assert_eq!(bridge.auth_mode(), AccountMode::ApiKeyBilling);
    assert_eq!(bridge.backend_id(), "backend.codex-app-server");
}

#[test]
fn local_codex_bridge_declares_local_app_session_auth() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));

    assert_eq!(bridge.provider_id(), None);
    assert_eq!(bridge.auth_mode(), AccountMode::LocalAppSession);
}

#[test]
fn codex_bridge_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/codex_bridge.rs",
        include_str!("../src/codex_bridge.rs"),
        250,
    )
    .expect("codex bridge source should stay below the initial line-count guard");
}
