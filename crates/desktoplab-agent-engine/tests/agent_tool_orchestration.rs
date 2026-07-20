use desktoplab_agent_engine::{
    AgentEvidence, AgentLoop, AgentRunRequest, ApprovalDecision, LlmExecutionAdapter,
    PlannedToolCall,
};
use desktoplab_agent_session::{SessionReplay, SessionState};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use xtask::check_logical_line_limit;

#[test]
fn unapproved_tool_call_blocks_session() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway);
    let request = AgentRunRequest::new("session.50", "backend.local").with_tool_call(
        PlannedToolCall::new(ToolIntent::filesystem_write("README.md")),
    );

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Blocked);
    assert_eq!(result.pending_approvals(), 1);
    assert!(result.event_names().contains(&"blocked"));
    assert!(
        result
            .session()
            .tool_decisions()
            .iter()
            .any(|decision| decision.contains("state=approval_required"))
    );
    assert!(
        result
            .session()
            .tool_decisions()
            .iter()
            .all(|decision| !decision.contains("state=executed"))
    );
}

#[test]
fn approved_filesystem_write_records_tool_result_and_diff() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Approved);
    let request = AgentRunRequest::new("session.51", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_write(
            "README.md",
        )))
        .with_diff("diff -- README.md");

    let result = loop_engine.run(request);

    assert!(result.evidence().contains(&AgentEvidence::ToolExecuted(
        "filesystem.write:README.md".into()
    )));
    assert!(
        result
            .session()
            .tool_decisions()
            .iter()
            .any(|decision| decision.contains("state=executed"))
    );
    assert!(
        result
            .session()
            .tool_decisions()
            .iter()
            .any(|decision| decision.contains("source=filesystem.write"))
    );
    assert!(
        result
            .evidence()
            .contains(&AgentEvidence::DiffCaptured("diff -- README.md".into()))
    );
}

#[test]
fn approved_mutating_tool_records_planned_approved_and_executed_states() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Approved);
    let request = AgentRunRequest::new("session.54", "backend.local").with_tool_call(
        PlannedToolCall::new(ToolIntent::filesystem_write("README.md")),
    );

    let result = loop_engine.run(request);

    let decisions = result.session().tool_decisions();
    assert!(
        decisions
            .iter()
            .any(|decision| decision.contains("event=before_tool"))
    );
    let lifecycle = decisions
        .iter()
        .filter(|decision| decision.starts_with("state="))
        .collect::<Vec<_>>();
    assert_eq!(lifecycle.len(), 3);
    assert!(lifecycle[0].contains("state=planned"));
    assert!(lifecycle[1].contains("state=approved"));
    assert!(lifecycle[2].contains("state=executed"));
}

#[test]
fn approved_terminal_test_records_output_and_status() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Approved);
    let request = AgentRunRequest::new("session.52", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::terminal("cargo test")))
        .with_test_result("status=0 stdout=ok");

    let result = loop_engine.run(request);

    assert!(
        result
            .evidence()
            .contains(&AgentEvidence::ToolExecuted("terminal:cargo test".into()))
    );
    assert!(
        result
            .evidence()
            .contains(&AgentEvidence::TestExecuted("status=0 stdout=ok".into()))
    );
}

#[test]
fn replayed_events_reconstruct_same_session_state() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Approved);
    let request = AgentRunRequest::new("session.53", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_write(
            "README.md",
        )))
        .with_diff("diff");

    let result = loop_engine.run(request);
    let replayed = SessionReplay::replay(result.events().to_vec()).expect("events should replay");

    assert_eq!(replayed.state(), result.session().state());
    assert_eq!(replayed.summary(), result.session().summary());
}

fn loop_with_backend(gateway: ToolGateway) -> AgentLoop {
    AgentLoop::new(gateway).with_backend_adapter(
        LlmExecutionAdapter::local("backend.local")
            .with_deterministic_output("Backend selected a tool step."),
    )
}

#[test]
fn agent_tool_orchestration_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/loop_engine.rs",
        include_str!("../src/loop_engine.rs"),
        300,
    )
    .expect("agent loop source should stay below the orchestration line-count guard");
}
