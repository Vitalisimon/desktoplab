use desktoplab_agent_engine::{
    AgentLoop, AgentRunRequest, LlmExecutionAdapter, LlmExecutionError, PlannedToolCall,
};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use xtask::check_logical_line_limit;

#[test]
fn agent_loop_uses_backend_output() {
    let backend = LlmExecutionAdapter::local("backend.ollama")
        .with_deterministic_output("Repo inspected by local adapter.");
    let mut loop_engine = AgentLoop::new(ToolGateway::new(PolicyEngine::default_conservative()))
        .with_backend_adapter(backend);

    let result = loop_engine.run(
        AgentRunRequest::new("session.real", "backend.ollama")
            .with_prompt("Inspect repository")
            .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_read(
                "README.md",
            ))),
    );

    assert_eq!(result.event_names()[2], "backend_response_received");
    assert_eq!(
        result.session().backend_responses()[0],
        "Repo inspected by local adapter."
    );
}

#[test]
fn agent_loop_sanitizes_backend_output_control_sequences() {
    let backend = LlmExecutionAdapter::local("backend.ollama")
        .with_deterministic_output("Questa miniapp\x1b[K gestisce contatti\x1b[10D\x1b[K.");
    let mut loop_engine = AgentLoop::new(ToolGateway::new(PolicyEngine::default_conservative()))
        .with_backend_adapter(backend);

    let result = loop_engine.run(
        AgentRunRequest::new("session.clean", "backend.ollama")
            .with_prompt("Descrivi questa miniapp"),
    );

    let response = &result.session().backend_responses()[0];
    assert!(!response.contains('\x1b'), "{response}");
    assert_eq!(response, "Questa miniapp gestisce contatti.");
}

#[test]
fn provider_egress_policy_blocks_backend_before_request() {
    let error = LlmExecutionAdapter::provider("provider.openai")
        .complete("Explain architecture")
        .expect_err("provider egress should require policy");

    assert_eq!(error, LlmExecutionError::ProviderEgressDenied);
}

#[test]
fn agent_engine_backend_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/llm.rs",
        include_str!("../src/llm.rs"),
        180,
    )
    .expect("llm adapter should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/loop_engine.rs",
        include_str!("../src/loop_engine.rs"),
        280,
    )
    .expect("agent loop should stay focused");
}
