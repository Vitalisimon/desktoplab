use desktoplab_agent_session::SessionOwner;
use desktoplab_backends::{ClaudeAgentSdkBridge, ClaudeBridgeConfig};
use desktoplab_domain::AccountMode;
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use xtask::check_logical_line_limit;

#[test]
fn claude_bridge_uses_external_backend_harness() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let session = bridge.create_session("session.claude");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.claude-agent-sdk");
}

#[test]
fn capability_differences_affect_routing() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["agent.sdk.claude", "approvals.boundary.external"]),
        vec![bridge.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Selected);
}

#[test]
fn missing_claude_specific_capability_blocks_route() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["codex.app-server"]),
        vec![bridge.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
}

#[test]
fn policy_and_approval_stay_in_desktoplab_for_claude_bridge() {
    let bridge = ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::new("claude-agent-sdk"));

    assert!(bridge.requires_desktoplab_policy());
    assert!(bridge.requires_desktoplab_approval_mapping());
}

#[test]
fn claude_bridge_declares_subscription_and_api_key_auth_modes() {
    let subscription =
        ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::subscription_account("claude-agent-sdk"));
    let api_key =
        ClaudeAgentSdkBridge::new(ClaudeBridgeConfig::api_key_billing("claude-agent-sdk"));

    assert_eq!(subscription.auth_mode(), AccountMode::SubscriptionAccount);
    assert_eq!(api_key.auth_mode(), AccountMode::ApiKeyBilling);
    assert!(subscription.requires_desktoplab_policy());
    assert!(api_key.requires_desktoplab_policy());
}

#[test]
fn claude_bridge_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/claude_bridge.rs",
        include_str!("../src/claude_bridge.rs"),
        250,
    )
    .expect("claude bridge source should stay below the initial line-count guard");
}
