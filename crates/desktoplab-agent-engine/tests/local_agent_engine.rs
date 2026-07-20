use desktoplab_agent_engine::{
    AgentEvidence, AgentLoop, AgentRunRequest, ApprovalDecision, LlmExecutionAdapter,
    PlannedToolCall,
};
use desktoplab_agent_session::SessionState;
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use xtask::check_logical_line_limit;

#[test]
fn agent_loop_cannot_execute_tools_outside_policy() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway);
    let request = AgentRunRequest::new("session.1", "backend.local").with_tool_call(
        PlannedToolCall::new(ToolIntent::filesystem_write("src/lib.rs")),
    );

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Blocked);
    assert_eq!(result.pending_approvals(), 1);
}

#[test]
fn approval_denial_blocks_execution() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Denied);
    let request = AgentRunRequest::new("session.2", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::terminal("cargo test")));

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Blocked);
    assert!(result.evidence().contains(&AgentEvidence::ApprovalDenied));
}

#[test]
fn diffs_and_test_evidence_attach_to_session_result() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = loop_with_backend(gateway).with_approval(ApprovalDecision::Approved);
    let request = AgentRunRequest::new("session.3", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_write(
            "README.md",
        )))
        .with_diff("diff -- README.md")
        .with_test_result("cargo test passed");

    let result = loop_engine.run(request);

    assert_eq!(result.session().state(), SessionState::Completed);
    assert!(
        result
            .evidence()
            .contains(&AgentEvidence::DiffCaptured("diff -- README.md".into()))
    );
    assert!(
        result
            .evidence()
            .contains(&AgentEvidence::TestExecuted("cargo test passed".into()))
    );
}

fn loop_with_backend(gateway: ToolGateway) -> AgentLoop {
    AgentLoop::new(gateway).with_backend_adapter(
        LlmExecutionAdapter::local("backend.local")
            .with_deterministic_output("Backend selected a policy-checked tool step."),
    )
}

#[test]
fn local_agent_engine_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-agent-engine/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-engine/src/loop_engine.rs",
            include_str!("../src/loop_engine.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-engine/src/request.rs",
            include_str!("../src/request.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("local agent engine source should stay below the initial line-count guard");
    }
}
