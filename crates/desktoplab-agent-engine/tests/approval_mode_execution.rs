use desktoplab_agent_engine::{AgentLoop, AgentRunRequest, PlannedToolCall};
use desktoplab_agent_session::SessionState;
use desktoplab_policy::{ApprovalMode, PolicyEngine};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};

#[test]
fn require_approval_blocks_mutating_agent_tools() {
    let result = run_with_mode(
        ApprovalMode::RequireApproval,
        ToolIntent::FilesystemWrite {
            path: "src/main.rs".to_string(),
        },
    );

    assert_eq!(result.session().state(), SessionState::Blocked);
    assert_eq!(result.pending_approvals(), 1);
    assert_eq!(
        result.session().blocked_reason(),
        Some("waiting for approval")
    );
    assert!(
        result.session().tool_decisions()[0].contains("approval_mode=require_approval"),
        "{:?}",
        result.session().tool_decisions()
    );
}

#[test]
fn approve_for_me_allows_safe_routine_work_but_not_push() {
    let write = run_with_mode(
        ApprovalMode::ApproveForMe,
        ToolIntent::FilesystemWrite {
            path: "src/main.rs".to_string(),
        },
    );
    assert_eq!(write.session().state(), SessionState::Completed);
    assert!(write.session().tool_decisions()[0].contains("approval_mode=approve_for_me"));

    let push = run_with_mode(
        ApprovalMode::ApproveForMe,
        ToolIntent::GitPush {
            remote: "origin".to_string(),
            branch: "main".to_string(),
        },
    );
    assert_eq!(push.session().state(), SessionState::Blocked);
    assert_eq!(push.pending_approvals(), 1);
}

#[test]
fn approve_workspace_writes_for_session_allows_workspace_writes_only() {
    let write = run_with_mode(
        ApprovalMode::ApproveWorkspaceWritesForSession,
        ToolIntent::FilesystemWrite {
            path: "src/main.rs".to_string(),
        },
    );
    assert_eq!(write.session().state(), SessionState::Completed);
    assert!(
        write.session().tool_decisions()[0]
            .contains("approval_mode=approve_workspace_writes_for_session")
    );

    let terminal = run_with_mode(
        ApprovalMode::ApproveWorkspaceWritesForSession,
        ToolIntent::terminal_workspace(
            "workspace.desktoplab",
            ".",
            "npm test",
            desktoplab_tool_gateway::TerminalRiskClass::Medium,
        ),
    );
    assert_eq!(terminal.session().state(), SessionState::Blocked);
    assert_eq!(terminal.pending_approvals(), 1);
}

#[test]
fn full_access_keeps_git_push_approval_and_local_only_blocks() {
    let push = run_with_mode(
        ApprovalMode::FullAccess,
        ToolIntent::GitPush {
            remote: "origin".to_string(),
            branch: "main".to_string(),
        },
    );
    assert_eq!(push.session().state(), SessionState::Blocked);
    assert_eq!(push.pending_approvals(), 1);

    let secret = run_with_mode(
        ApprovalMode::FullAccess,
        ToolIntent::FilesystemRead {
            path: ".env".to_string(),
        },
    );
    assert_eq!(secret.session().state(), SessionState::Blocked);
    assert_eq!(secret.session().blocked_reason(), Some("local_only_path"));
}

#[test]
fn approval_mode_execution_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-agent-engine/tests/approval_mode_execution.rs",
        include_str!("approval_mode_execution.rs"),
        190,
    )
    .expect("approval mode execution test should stay focused");
}

fn run_with_mode(
    mode: ApprovalMode,
    intent: ToolIntent,
) -> desktoplab_agent_engine::AgentRunResult {
    let gateway = ToolGateway::new(PolicyEngine::default_conservative().with_approval_mode(mode));
    let mut loop_engine = AgentLoop::new(gateway);
    loop_engine.run(
        AgentRunRequest::new("session.mode", "backend.local")
            .with_backend_response("I will use the requested tool.")
            .with_tool_call(PlannedToolCall::new(intent)),
    )
}
