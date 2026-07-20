use desktoplab_agent_session::{SessionEvent, SessionOwner, SessionReplay, SessionState};
use desktoplab_backends::{
    BridgeCallFailure, ClaudeAgentSdkBridge, ClaudeBridgeConfig, CodexAppServerBridge,
    CodexBridgeConfig, ExternalEvent,
};
use desktoplab_domain::ProviderId;
use xtask::check_logical_line_limit;

#[test]
fn codex_bridge_imports_events_but_desktoplab_owns_session_state() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::local("http://127.0.0.1:1455"));
    let imported = bridge.import_events(
        "session.codex.product",
        vec![
            ExternalEvent::text_delta("plan created"),
            ExternalEvent::Completed("done".into()),
        ],
    );

    assert_eq!(imported.session().owner(), SessionOwner::DesktopLab);
    assert_eq!(imported.session().state(), SessionState::Completed);
    assert!(imported.events().contains(&SessionEvent::completed("done")));
}

#[test]
fn codex_bridge_redacts_provider_backend_auth_in_diagnostics() {
    let bridge = CodexAppServerBridge::new(CodexBridgeConfig::with_provider(
        "https://codex.example",
        ProviderId::new("provider.openai"),
    ));

    let diagnostic = bridge.redacted_auth_diagnostic("Bearer sk-secret-token");

    assert!(diagnostic.contains("[REDACTED]"));
    assert!(!diagnostic.contains("sk-secret-token"));
}

#[test]
fn claude_bridge_failure_blocks_session_with_evidence() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let blocked = bridge.record_failure(
        "session.claude.failed",
        BridgeCallFailure::new("timeout", "SDK timeout"),
    );

    assert_eq!(blocked.session().state(), SessionState::Blocked);
    assert!(blocked.evidence().contains("timeout"));
    assert!(blocked.evidence().contains("SDK timeout"));
}

#[test]
fn claude_bridge_normalizes_events_like_desktoplab_session_events() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let imported = bridge.import_events(
        "session.claude.product",
        vec![
            ExternalEvent::text_delta("thinking"),
            ExternalEvent::Completed("ok".into()),
        ],
    );
    let replayed = SessionReplay::replay(imported.events().to_vec()).unwrap();

    assert_eq!(replayed.owner(), SessionOwner::DesktopLab);
    assert_eq!(replayed.state(), SessionState::Completed);
}

#[test]
fn external_bridge_productization_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/productization.rs",
        include_str!("../src/productization.rs"),
        260,
    )
    .expect("external bridge productization source should stay focused");
}
