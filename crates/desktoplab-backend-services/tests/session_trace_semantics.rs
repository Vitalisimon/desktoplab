use desktoplab_agent_session::{SessionEvent, TerminalEvidence};
use desktoplab_backend_services::{SessionService, SessionServiceStore};
use xtask::check_logical_line_limit;

#[test]
fn trace_distinguishes_policy_approval_from_executed_mutation() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.desktoplab", "backend.ollama");
    service.append_events(
        session.session_id(),
        &[
            SessionEvent::tool_decision_recorded(
                "state=approved source=filesystem.write tool=filesystem.write:notes.md approval_mode=require_approval",
            ),
            SessionEvent::tool_decision_recorded(
                "state=executed source=filesystem.write tool=filesystem.write:notes.md approval_mode=require_approval",
            ),
            SessionEvent::tool_decision_recorded(
                "state=observed source=filesystem.write tool=filesystem.write:notes.md approval_mode=require_approval mutation=false",
            ),
        ],
    );

    let trace = service
        .trace(session.session_id())
        .expect("trace should exist");
    let approval = trace.events()[1].to_value();
    let execution = trace.events()[2].to_value();
    assert_eq!(approval["kind"], "approval_resolved");
    assert_eq!(approval["mutation"], false);
    assert_eq!(execution["kind"], "tool_observed");
    assert_eq!(execution["mutation"], true);
    assert_eq!(execution["success"], true);
    assert!(!trace.events()[3].to_value()["mutation"].as_bool().unwrap());
}

#[test]
fn trace_identifies_filesystem_patch_as_a_mutating_canonical_tool() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.desktoplab", "backend.ollama");
    service.append_events(
        session.session_id(),
        &[SessionEvent::tool_decision_recorded(
            "state=executed source=filesystem.patch tool=filesystem.patch:notes.md approval_mode=require_approval",
        )],
    );

    let trace = service
        .trace(session.session_id())
        .expect("trace should exist");
    let execution = trace.events()[1].to_value();
    assert_eq!(execution["kind"], "tool_observed");
    assert_eq!(execution["source"], "desktoplab.patch_file");
    assert_eq!(execution["mutation"], true);
    assert_eq!(execution["success"], true);
}

#[test]
fn terminal_result_observation_does_not_double_count_the_executed_mutation() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.desktoplab", "backend.ollama");
    service.append_events(
        session.session_id(),
        &[
            SessionEvent::tool_decision_recorded(
                "state=executed source=terminal.agent_command tool=terminal:npm-test approval_mode=require_approval",
            ),
            SessionEvent::terminal_evidence_recorded(TerminalEvidence::new(
                "npm test",
                "PASS",
                Some(0),
            )),
        ],
    );

    let trace = service
        .trace(session.session_id())
        .expect("trace should exist");
    assert_eq!(trace.events()[1].to_value()["mutation"], true);
    assert_eq!(trace.events()[2].kind(), "terminal_observed");
    assert_eq!(trace.events()[2].to_value()["mutation"], false);
    assert_eq!(
        trace
            .events()
            .iter()
            .filter(|event| event.to_value()["mutation"] == true)
            .count(),
        1
    );
}

#[test]
fn session_trace_semantics_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/session_trace_semantics.rs",
        include_str!("session_trace_semantics.rs"),
        95,
    )
    .expect("session trace semantics test should stay focused");
}
