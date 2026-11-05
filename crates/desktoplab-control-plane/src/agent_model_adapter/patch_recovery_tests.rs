use super::{backend_messages, suppressed_tool_for_model_turn};
use desktoplab_agent_engine::{DesktopLabToolRegistry, IterativeLoopState};
use desktoplab_backends::BackendMessage;

#[test]
fn patch_recovery_tests_stay_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_model_adapter/patch_recovery_tests.rs",
        include_str!("patch_recovery_tests.rs"),
        80,
    )
    .unwrap();
}

#[test]
fn patch_without_prior_read_temporarily_hides_patch_but_preserves_read() {
    let mut state = IterativeLoopState::new("session.patch-recovery");
    assert!(state.request_model_protocol_retry("patch_requires_prior_read"));
    assert_eq!(
        suppressed_tool_for_model_turn(&state),
        Some("desktoplab.patch_file")
    );
    let messages = backend_messages(
        "Inspect the workspace",
        &state,
        &DesktopLabToolRegistry::default(),
    );
    let BackendMessage::User(message) = messages.last().unwrap() else {
        panic!("expected protocol feedback")
    };
    let tools = message
        .split("Canonical names available for this turn:")
        .nth(1)
        .unwrap();
    assert!(tools.contains("desktoplab.read_file"));
    assert!(!tools.contains("desktoplab.patch_file"));
    state.clear_model_protocol_recovery();
    assert_eq!(suppressed_tool_for_model_turn(&state), None);
}
