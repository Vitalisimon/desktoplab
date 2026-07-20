use desktoplab_agent_engine::{
    AgentContextBuilder, AgentPlanStore, AgentPlanner, AgentRunRequest, ApprovalDecision,
    ExecutionBackendAvailability, FileEditEngine, LlmExecutionAdapter, PlannedToolCall,
    SessionControl, TestFeedbackLoop,
};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn context_builder_excludes_local_only_files_enforces_budget_and_records_provenance() {
    let context = AgentContextBuilder::new(48)
        .with_workspace_fact("language=rust", "repository inspection")
        .with_file("src/lib.rs", "pub fn demo() {}\n", false)
        .with_file(".env", "SECRET=value\n", true)
        .build();

    assert!(context.text().contains("language=rust"));
    assert!(context.text().contains("src/lib.rs"));
    assert!(!context.text().contains("SECRET=value"));
    assert!(context.text().len() <= 48);
    assert!(
        context
            .provenance()
            .contains(&"repository inspection".to_string())
    );
}

#[test]
fn llm_execution_adapter_normalizes_streams_and_checks_egress_before_request() {
    let local = LlmExecutionAdapter::local("backend.ollama")
        .with_deterministic_output("local test output")
        .complete("plan")
        .unwrap();
    assert_eq!(local.backend_id(), "backend.ollama");
    assert_eq!(
        local.events(),
        &["stream_started", "delta", "stream_completed"]
    );

    let denied = LlmExecutionAdapter::provider("provider.openai")
        .with_provider_egress_allowed(false)
        .complete("plan");
    assert!(denied.is_err());

    let cancelled = LlmExecutionAdapter::local("backend.ollama")
        .with_deterministic_output("cancelled test output")
        .cancel_before_complete()
        .complete("plan")
        .unwrap();
    assert_eq!(cancelled.events().last(), Some(&"cancelled"));
}

#[test]
fn planning_loop_persists_plan_and_blocks_when_backend_unavailable() {
    let store = AgentPlanStore::default();
    let planner = AgentPlanner::new(store.clone());

    let planned = planner.plan(
        "session.plan",
        "build feature",
        ExecutionBackendAvailability::Available("backend.ollama".into()),
    );
    assert_eq!(planned.status(), "planned");
    assert!(store.get("session.plan").unwrap().contains("build feature"));
    assert!(planned.events().contains(&"plan_event"));

    let blocked = planner.plan(
        "session.blocked",
        "build feature",
        ExecutionBackendAvailability::Unavailable("no backend".into()),
    );
    assert_eq!(blocked.status(), "blocked");
    assert_eq!(blocked.next_action(), Some("configure execution backend"));
}

#[test]
fn tool_call_loop_approval_and_result_events_remain_desktoplab_owned() {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut loop_engine = desktoplab_agent_engine::AgentLoop::new(gateway)
        .with_approval(ApprovalDecision::Approved)
        .with_backend_adapter(
            LlmExecutionAdapter::local("backend.local")
                .with_deterministic_output("Backend proposed a README edit."),
        );
    let request = AgentRunRequest::new("session.tool", "backend.local")
        .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_write(
            "README.md",
        )))
        .with_diff("diff -- README.md");

    let result = loop_engine.run(request);

    assert!(result.event_names().contains(&"planning_started"));
    assert!(result.event_names().contains(&"completed"));
    assert_eq!(result.pending_approvals(), 0);
}

#[test]
fn file_edit_engine_rejects_outside_workspace_blocks_conflicts_and_records_diff() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("README.md"), "before\n").unwrap();

    let engine = FileEditEngine::new(workspace.path());
    assert!(
        engine
            .apply("../outside.txt", "", "malicious")
            .unwrap_err()
            .contains("outside_workspace")
    );

    fs::write(workspace.path().join("README.md"), "changed elsewhere\n").unwrap();
    assert!(
        engine
            .apply("README.md", "before\n", "after\n")
            .unwrap_err()
            .contains("conflict")
    );

    fs::write(workspace.path().join("README.md"), "before\n").unwrap();
    let applied = engine.apply("README.md", "before\n", "after\n").unwrap();
    assert!(applied.diff_evidence().contains("-before"));
    assert!(applied.diff_evidence().contains("+after"));
}

#[test]
fn test_feedback_loop_requires_approval_bounds_output_and_redacts_tokens() {
    let feedback = TestFeedbackLoop::new(24);

    let pending = feedback.capture("cargo test", None, "ok");
    assert_eq!(pending.status(), "approval_required");

    let approved = feedback.capture(
        "cargo test",
        Some(ApprovalDecision::Approved),
        "failed TOKEN=secret-token with a very long stdout payload",
    );
    assert_eq!(approved.status(), "captured");
    assert!(approved.summary().contains("[REDACTED]"));
    assert!(approved.summary().len() <= 24);
}

#[test]
fn test_feedback_loop_proposes_terminal_test_command_with_pending_approval() {
    let feedback = TestFeedbackLoop::new(128);

    let proposal = feedback.propose_command("workspace.desktoplab", "cargo test");

    assert_eq!(proposal.command(), "cargo test");
    assert_eq!(
        proposal.terminal_request().workspace_id(),
        "workspace.desktoplab"
    );
    assert_eq!(proposal.terminal_request().command(), "cargo test");
    assert_eq!(
        proposal.terminal_request().approval_state(),
        desktoplab_tool_gateway::TerminalApproval::Pending
    );
}

#[test]
fn session_controls_pause_resume_and_cancel_mutation_rules() {
    let mut controls = SessionControl::new("session.controls");

    controls.pause();
    assert!(!controls.can_execute_tools());
    controls.resume();
    assert!(controls.can_execute_tools());
    controls.cancel();
    assert!(!controls.can_mutate_files());
}

#[test]
fn agent_productization_sources_stay_below_line_count_guards() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-agent-engine/src/context.rs",
            include_str!("../src/context.rs"),
            260,
        ),
        (
            "crates/desktoplab-agent-engine/src/llm.rs",
            include_str!("../src/llm.rs"),
            260,
        ),
        (
            "crates/desktoplab-agent-engine/src/product_loop.rs",
            include_str!("../src/product_loop.rs"),
            320,
        ),
        (
            "crates/desktoplab-agent-engine/src/prompt_step.rs",
            include_str!("../src/prompt_step.rs"),
            100,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("agent productization modules should stay focused");
    }
}
