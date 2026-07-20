use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use desktoplab_agent_engine::{
    AgentEvidence, AgentLoop, AgentRunRequest, ApprovalDecision, LlmExecutionAdapter,
    PlannedToolCall,
};
use desktoplab_agent_session::{SessionEvent, SessionReplay, SessionState};
use desktoplab_backends::{BackendModelInventory, OllamaExecutionBackend};
use desktoplab_domain::WorkspaceId;
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{ToolGateway, ToolIntent};
use desktoplab_workspace::{GitRepository, WorkspaceRegistration, WorkspaceRegistry};

#[test]
fn backend_core_can_support_future_ui_without_frontend_mocks() {
    let repo_path = create_fixture_repo();
    let mut registry = WorkspaceRegistry::default();
    let workspace_id = WorkspaceId::new("workspace.e2e");
    registry.register(WorkspaceRegistration::new(
        workspace_id.clone(),
        repo_path.clone(),
    ));
    assert!(registry.get(&workspace_id).is_some());

    let repo = GitRepository::open(&repo_path).expect("repo should open");
    assert!(!repo.status().expect("status should read").is_dirty());

    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["qwen3:8b"]));
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["llm.chat", "runtime.ollama"]),
        vec![backend.route_candidate()],
    );
    assert_eq!(route.status(), RouteStatus::Selected);

    fs::write(
        repo_path.join("README.md"),
        "# DesktopLab\n\nchanged by agent\n",
    )
    .expect("fixture write should work");
    let diff = repo.diff().expect("diff should read").as_text().to_string();
    assert!(diff.contains("changed by agent"));
    assert!(
        Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&repo_path)
            .output()
            .expect("test command should run")
            .status
            .success()
    );

    let gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let mut agent_loop = AgentLoop::new(gateway)
        .with_approval(ApprovalDecision::Approved)
        .with_backend_adapter(
            LlmExecutionAdapter::local(route.backend_id().expect("selected backend"))
                .with_deterministic_output("Backend accepted fixture execution."),
        );
    let result = agent_loop.run(
        AgentRunRequest::new("session.e2e", route.backend_id().expect("selected backend"))
            .with_tool_call(PlannedToolCall::new(ToolIntent::filesystem_write(
                "README.md",
            )))
            .with_tool_call(PlannedToolCall::new(ToolIntent::terminal(
                "git status --porcelain",
            )))
            .with_diff(diff)
            .with_test_result("git status command passed"),
    );

    assert_eq!(result.session().state(), SessionState::Completed);
    assert!(result.evidence().iter().any(|evidence| {
        matches!(evidence, AgentEvidence::DiffCaptured(diff) if diff.contains("changed by agent"))
    }));
    assert!(result.evidence().contains(&AgentEvidence::TestExecuted(
        "git status command passed".to_string()
    )));

    let replayed = SessionReplay::replay(vec![
        SessionEvent::created("session.e2e", route.backend_id().unwrap()),
        SessionEvent::planning_started("plan accepted for execution"),
        SessionEvent::execution_started(),
        SessionEvent::completed("agent loop completed"),
    ])
    .expect("session should replay");
    assert_eq!(replayed.state(), SessionState::Completed);
}

fn create_fixture_repo() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "desktoplab-e2e-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("fixture dir should exist");
    run_git(&path, &["init"]);
    fs::write(path.join("README.md"), "# DesktopLab\n").expect("fixture file should write");
    run_git(&path, &["add", "README.md"]);
    run_git(
        &path,
        &[
            "-c",
            "user.name=DesktopLab Test",
            "-c",
            "user.email=test@desktoplab.local",
            "commit",
            "-m",
            "initial",
        ],
    );
    path
}

fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
