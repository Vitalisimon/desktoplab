use desktoplab_agent_session::{SessionEvent, SessionReplay};

use super::session_failure_payload;

#[test]
fn classification_preserves_original_reason_and_stable_user_copy() {
    let payload = failure_payload("session.1", "repeated_tool_failure");
    assert_eq!(payload["primary"], "repeated_error_loop");
    assert_eq!(payload["originalStopReason"], "repeated_tool_failure");
    assert_eq!(
        payload["userMessage"],
        "The agent repeated the same failing action without progress."
    );
}

#[test]
fn native_protocol_failure_keeps_the_specific_tool_error() {
    let reason = "model_failure:model_protocol_error:unknown_tool:read_file";
    let payload = failure_payload("session.2", reason);
    assert_eq!(payload["primary"], "tool_misuse");
    assert_eq!(payload["originalStopReason"], reason);
}

#[test]
fn unsupported_test_claim_is_reported_as_unproven_completion() {
    let payload = failure_payload("session.3", "unsupported_test_claim");
    assert_eq!(payload["primary"], "hallucinated_completion");
    assert_eq!(
        payload["userMessage"],
        "The agent claimed completion without executable proof."
    );
}

#[test]
fn failed_validation_has_actionable_user_copy() {
    let payload = failure_payload("session.4", "tests_failed:1");
    assert_eq!(payload["primary"], "validation_failed");
    assert_eq!(
        payload["userMessage"],
        "The latest validation command failed. Review the output, repair the issue, and run it again."
    );
}

#[test]
fn local_model_transport_failure_has_actionable_user_copy() {
    let payload = failure_payload(
        "session.transport",
        "model_failure:ollama_request_failed:error sending request for url",
    );
    assert_eq!(payload["primary"], "model_transport_failure");
    assert_eq!(
        payload["userMessage"],
        "The local model runner stopped responding. Check that it is running, then retry the turn."
    );
}

#[test]
fn persisted_failed_validation_overrides_a_generic_final_stop_reason() {
    let session = SessionReplay::replay(vec![
        SessionEvent::created("session.5", "backend.ollama"),
        SessionEvent::tool_decision_recorded(
            "state=failed source=agent.iterative canonical=desktoplab.run_tests tool=desktoplab.run_tests call_id=test-1",
        ),
        SessionEvent::terminal_evidence_recorded(desktoplab_agent_session::TerminalEvidence::new(
            "node test.js",
            "assertion failed",
            Some(1),
        )),
        SessionEvent::failed("model decisions exhausted"),
    ])
    .unwrap();
    let payload = session_failure_payload(&session);
    assert_eq!(payload["primary"], "validation_failed");
    assert_eq!(payload["originalStopReason"], "model decisions exhausted");
}

#[test]
fn a_later_passing_validation_clears_the_earlier_failure() {
    let session = SessionReplay::replay(vec![
        SessionEvent::created("session.6", "backend.ollama"),
        validation_event("failed", "test-1"),
        terminal_event(1),
        validation_event("observed", "test-2"),
        terminal_event(0),
        SessionEvent::failed("model decisions exhausted"),
    ])
    .unwrap();
    assert_eq!(session_failure_payload(&session)["primary"], "unclassified");
}

fn validation_event(state: &str, id: &str) -> SessionEvent {
    SessionEvent::tool_decision_recorded(format!(
        "state={state} source=agent.iterative canonical=desktoplab.run_tests tool=desktoplab.run_tests call_id={id}"
    ))
}

fn terminal_event(code: i32) -> SessionEvent {
    SessionEvent::terminal_evidence_recorded(desktoplab_agent_session::TerminalEvidence::new(
        "node test.js",
        "",
        Some(code),
    ))
}

#[test]
fn failure_classifier_stays_below_line_guard() {
    let logical = include_str!("agent_failure.rs")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert!(
        logical <= 240,
        "agent_failure.rs has {logical} logical lines"
    );
}

fn failure_payload(session_id: &str, reason: &str) -> serde_json::Value {
    let session = SessionReplay::replay(vec![
        SessionEvent::created(session_id, "backend.ollama"),
        SessionEvent::failed(reason),
    ])
    .unwrap();
    session_failure_payload(&session)
}
