use std::time::Duration;

use desktoplab_agent_engine::{
    ApprovalDecision, BoundedToolLoop, ToolLoopLimits, ToolLoopStep, ToolLoopStopReason,
};
use desktoplab_agent_session::{AgentSession, SessionEvent, SessionState};
use desktoplab_policy::{ApprovalMode, PolicyEngine};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use xtask::check_logical_line_limit;

#[test]
fn bounded_loop_records_observation_before_next_tool_step() {
    let mut session = AgentSession::new("session.loop", "backend.local");
    let mut events = vec![SessionEvent::created("session.loop", "backend.local")];
    let mut loop_engine = BoundedToolLoop::new(
        ToolGateway::new(PolicyEngine::default_conservative()),
        ToolLoopLimits::new(6, 6, Duration::from_secs(5)),
    );

    let result = loop_engine.run(
        &mut session,
        &mut events,
        &[
            ToolLoopStep::new(ToolIntent::filesystem_read("README.md"))
                .with_observation("README contents"),
            ToolLoopStep::new(ToolIntent::filesystem_read("src/lib.rs"))
                .with_observation("lib contents"),
        ],
        ApprovalDecision::Pending,
    );

    assert_eq!(result.stop_reason(), None);
    assert_eq!(
        session.backend_responses()[0],
        "Observation: README contents"
    );
    assert!(
        events.iter().position(is_readme_observation).unwrap()
            < events.iter().position(is_second_tool_plan).unwrap()
    );
}

#[test]
fn approval_required_blocks_with_resumable_stop_reason() {
    let mut session = AgentSession::new("session.approval", "backend.local");
    let mut events = vec![SessionEvent::created("session.approval", "backend.local")];
    let mut loop_engine = BoundedToolLoop::new(
        ToolGateway::new(PolicyEngine::default_conservative()),
        ToolLoopLimits::new(4, 4, Duration::from_secs(5)),
    );

    let result = loop_engine.run(
        &mut session,
        &mut events,
        &[ToolLoopStep::new(ToolIntent::filesystem_write("notes.md"))],
        ApprovalDecision::Pending,
    );

    assert_eq!(result.pending_approvals(), 1);
    assert_eq!(
        result.stop_reason(),
        Some(ToolLoopStopReason::ApprovalRequired)
    );
    assert_eq!(session.state(), SessionState::Blocked);
    assert_eq!(session.blocked_reason(), Some("waiting for approval"));
}

#[test]
fn bounded_loop_stops_without_claiming_completion() {
    let mut session = AgentSession::new("session.max", "backend.local");
    let mut events = vec![SessionEvent::created("session.max", "backend.local")];
    let mut loop_engine = BoundedToolLoop::new(
        ToolGateway::new(
            PolicyEngine::default_conservative().with_approval_mode(ApprovalMode::ApproveForMe),
        ),
        ToolLoopLimits::new(8, 1, Duration::from_secs(5)),
    );

    let result = loop_engine.run(
        &mut session,
        &mut events,
        &[
            ToolLoopStep::new(ToolIntent::filesystem_write("a.md")),
            ToolLoopStep::new(ToolIntent::filesystem_write("b.md")),
        ],
        ApprovalDecision::Approved,
    );

    assert_eq!(result.stop_reason(), Some(ToolLoopStopReason::MaxToolCalls));
    assert_eq!(session.state(), SessionState::Blocked);
    assert!(session.summary().is_none());
}

#[test]
fn bounded_loop_has_step_and_duration_stop_reasons() {
    let mut step_session = AgentSession::new("session.steps", "backend.local");
    let mut step_events = vec![SessionEvent::created("session.steps", "backend.local")];
    let mut step_loop = BoundedToolLoop::new(
        ToolGateway::new(PolicyEngine::default_conservative()),
        ToolLoopLimits::new(0, 8, Duration::from_secs(5)),
    );
    let step_result = step_loop.run(
        &mut step_session,
        &mut step_events,
        &[ToolLoopStep::new(ToolIntent::filesystem_read("README.md"))],
        ApprovalDecision::Pending,
    );

    let mut time_session = AgentSession::new("session.time", "backend.local");
    let mut time_events = vec![SessionEvent::created("session.time", "backend.local")];
    let mut time_loop = BoundedToolLoop::new(
        ToolGateway::new(PolicyEngine::default_conservative()),
        ToolLoopLimits::new(8, 8, Duration::from_millis(0)),
    );
    let time_result = time_loop.run(
        &mut time_session,
        &mut time_events,
        &[ToolLoopStep::new(ToolIntent::filesystem_read("README.md"))],
        ApprovalDecision::Pending,
    );

    assert_eq!(
        step_result.stop_reason(),
        Some(ToolLoopStopReason::MaxSteps)
    );
    assert_eq!(
        time_result.stop_reason(),
        Some(ToolLoopStopReason::MaxDuration)
    );
}

#[test]
fn bounded_tool_loop_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-agent-engine/src/tool_loop.rs",
            include_str!("../src/tool_loop.rs"),
            260,
        ),
        (
            "crates/desktoplab-agent-engine/tests/bounded_tool_loop.rs",
            include_str!("bounded_tool_loop.rs"),
            170,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("bounded tool loop files should stay focused");
    }
}

fn is_readme_observation(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::BackendResponseReceived { message } if message.contains("README contents"))
}

fn is_second_tool_plan(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::ToolDecisionRecorded { decision } if decision.contains("filesystem.read:src/lib.rs") && decision.contains("state=planned"))
}
