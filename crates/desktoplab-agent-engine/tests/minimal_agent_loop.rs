use desktoplab_agent_engine::{AgentLoop, AgentRunRequest, PlannedToolCall};
use desktoplab_agent_session::SessionState;
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use xtask::check_logical_line_limit;

#[test]
fn minimal_loop_consumes_prompt_and_records_backend_response_before_tool_decision() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = AgentLoop::new(gateway);
    let request = AgentRunRequest::new("session.1", "backend.ollama")
        .with_prompt("Inspect the repo and suggest first step")
        .with_backend_response("I will inspect files first.")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_read(
            "README.md",
        )));

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Completed);
    assert_eq!(
        result.event_names(),
        vec![
            "created",
            "planning_started",
            "backend_response_received",
            "execution_started",
            "tool_decision",
            "tool_decision",
            "tool_decision",
            "backend_response_received",
            "completed"
        ]
    );
    assert!(
        result.session().backend_responses()[1]
            .contains("Observation: tool filesystem.read:README.md completed")
    );
    assert_eq!(
        result.session().plan(),
        Some("Inspect the repo and suggest first step")
    );
}

#[test]
fn minimal_loop_records_blocked_tool_request_as_session_event() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = AgentLoop::new(gateway);
    let request = AgentRunRequest::new("session.2", "backend.ollama")
        .with_prompt("Read local env")
        .with_backend_response("I need to inspect the environment file.")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_read(".env")));

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Blocked);
    assert_eq!(
        result.event_names(),
        vec![
            "created",
            "planning_started",
            "backend_response_received",
            "execution_started",
            "tool_decision",
            "tool_decision",
            "tool_decision",
            "blocked"
        ]
    );
}

#[test]
fn minimal_agent_loop_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/tests/minimal_agent_loop.rs",
        include_str!("minimal_agent_loop.rs"),
        150,
    )
    .expect("minimal agent loop test should stay focused");
}
