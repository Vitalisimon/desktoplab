use std::time::Duration;

use desktoplab_agent_engine::{
    ApprovalDecision, BoundedToolLoop, ToolLoopLimits, ToolLoopStep, ToolLoopStopReason,
};
use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_policy::{ApprovalMode, PolicyEngine};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};

#[test]
fn before_tool_event_is_bounded_redacted_and_policy_explicit() {
    let mut session = AgentSession::new("session.telemetry", "backend.local");
    let mut events = vec![SessionEvent::created("session.telemetry", "backend.local")];
    let mut loop_engine = BoundedToolLoop::new(
        ToolGateway::new(PolicyEngine::default_conservative()),
        ToolLoopLimits::new(4, 4, Duration::from_secs(5)),
    );

    let result = loop_engine.run(
        &mut session,
        &mut events,
        &[ToolLoopStep::new(ToolIntent::terminal(
            "echo token=sk-secret Authorization: Bearer raw-secret",
        ))],
        ApprovalDecision::Approved,
    );

    assert_eq!(result.stop_reason(), None);
    let event = before_tool_event(&events);
    assert!(event.contains("ordinal=1"));
    assert!(event.contains("source=terminal.agent_command"));
    assert!(event.contains("risk=medium"));
    assert!(event.contains("policy_result=requires_approval"));
    assert!(event.contains("approval_state=approved"));
    assert!(event.len() <= 360);
    assert!(!event.contains("sk-secret"));
    assert!(!event.contains("raw-secret"));
}

#[test]
fn repeated_identical_failures_stop_the_loop_and_redact_observations() {
    let mut session = AgentSession::new("session.repeat", "backend.local");
    let mut events = vec![SessionEvent::created("session.repeat", "backend.local")];
    let mut loop_engine = BoundedToolLoop::new(
        ToolGateway::new(
            PolicyEngine::default_conservative().with_approval_mode(ApprovalMode::ApproveForMe),
        ),
        ToolLoopLimits::new(8, 8, Duration::from_secs(5)),
    );
    let failing_step = || {
        ToolLoopStep::new(ToolIntent::filesystem_read("missing.md"))
            .with_observation("error: file not found token=sk-secret")
    };

    let result = loop_engine.run(
        &mut session,
        &mut events,
        &[failing_step(), failing_step(), failing_step()],
        ApprovalDecision::Pending,
    );

    assert_eq!(
        result.stop_reason(),
        Some(ToolLoopStopReason::RepeatedToolFailure)
    );
    assert_eq!(
        session.blocked_reason(),
        Some("repeated_identical_tool_failure")
    );
    assert!(
        session
            .backend_responses()
            .iter()
            .all(|message| !message.contains("sk-secret"))
    );
}

fn before_tool_event(events: &[SessionEvent]) -> &str {
    events
        .iter()
        .find_map(|event| match event {
            SessionEvent::ToolDecisionRecorded { decision }
                if decision.contains("event=before_tool") =>
            {
                Some(decision.as_str())
            }
            _ => None,
        })
        .expect("before-tool telemetry event should be recorded")
}
