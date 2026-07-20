use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    TerminalApproval, TerminalCommandRequest, TerminalExecutionStatus, TerminalOutputEvent,
    TerminalRiskClass, TerminalToolExecutor, TerminalToolOutcome, ToolIntent,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn terminal_execution_requires_approval_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", write_stdout("hello")),
        TerminalApproval::Pending,
    );

    assert_eq!(outcome, TerminalToolOutcome::ApprovalRequired);
    assert_eq!(executor.approval_count(), 1);
    assert_eq!(executor.audit_count(), 1);
}

#[test]
fn denied_approval_does_not_spawn_process() {
    let temp_dir = TempDir::new().unwrap();
    let marker = temp_dir.path().join("marker.txt");
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", write_marker()),
        TerminalApproval::Denied,
    );

    assert_eq!(outcome, TerminalToolOutcome::Denied);
    assert!(!marker.exists());
}

#[test]
fn timeout_kills_command_and_records_failure() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_millis(50),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", slow_command()),
        TerminalApproval::Approved,
    );

    let TerminalToolOutcome::Completed(result) = outcome else {
        panic!("terminal command should return a result");
    };
    assert_eq!(result.status(), TerminalExecutionStatus::TimedOut);
    assert_eq!(executor.audit_count(), 1);
}

#[test]
fn output_capture_is_bounded_and_redacted() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        12,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new(
            "workspace.fixture",
            write_stdout("token=abc123456789 long-output"),
        ),
        TerminalApproval::Approved,
    );

    let TerminalToolOutcome::Completed(result) = outcome else {
        panic!("terminal command should return a result");
    };
    assert_eq!(result.status(), TerminalExecutionStatus::Exited(0));
    assert_eq!(result.stdout(), "token=[REDAC");
}

#[test]
fn working_directory_escape_is_blocked() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", "pwd").with_working_directory("../"),
        TerminalApproval::Approved,
    );

    assert_eq!(outcome, TerminalToolOutcome::Blocked("path_escape"));
}

#[test]
fn approved_terminal_command_runs_in_workspace() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("input.txt"), "ok").unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", read_input()),
        TerminalApproval::Approved,
    );

    let TerminalToolOutcome::Completed(result) = outcome else {
        panic!("terminal command should return a result");
    };
    assert_eq!(result.status(), TerminalExecutionStatus::Exited(0));
    assert_eq!(result.stdout(), "ok");
}

#[test]
fn terminal_request_carries_workspace_scope_risk_and_approval_state() {
    let request = TerminalCommandRequest::for_workspace("workspace.desktoplab", "npm test")
        .with_working_directory("apps/desktop")
        .with_risk_class(TerminalRiskClass::High)
        .with_approval_state(TerminalApproval::Pending);

    assert_eq!(request.workspace_id(), "workspace.desktoplab");
    assert_eq!(request.working_directory(), PathBuf::from("apps/desktop"));
    assert_eq!(request.command(), "npm test");
    assert_eq!(request.risk_class(), TerminalRiskClass::High);
    assert_eq!(request.approval_state(), TerminalApproval::Pending);
}

#[test]
fn terminal_intent_records_workspace_working_directory_and_risk() {
    let intent = ToolIntent::terminal_workspace(
        "workspace.desktoplab",
        "apps/desktop",
        "npm test",
        TerminalRiskClass::Medium,
    );

    assert_eq!(intent.terminal_workspace_id(), Some("workspace.desktoplab"));
    assert_eq!(intent.terminal_working_directory(), Some("apps/desktop"));
    assert_eq!(
        intent.terminal_risk_class(),
        Some(TerminalRiskClass::Medium)
    );
}

#[test]
fn terminal_output_event_is_redacted_and_workspace_scoped() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor = TerminalToolExecutor::new(
        temp_dir.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );
    let request =
        TerminalCommandRequest::for_workspace("workspace.desktoplab", write_stdout("secret=abc"));

    let TerminalToolOutcome::Completed(result) =
        executor.execute(request.clone(), TerminalApproval::Approved)
    else {
        panic!("terminal command should return a result");
    };
    let event = TerminalOutputEvent::from_result("terminal.1", &request, &result);

    assert_eq!(event.workspace_id(), "workspace.desktoplab");
    assert_eq!(event.terminal_id(), "terminal.1");
    assert!(event.redacted());
    assert!(event.stdout().contains("secret=[REDACTED]"));
    assert!(!event.stdout().contains("abc"));
}

#[cfg(not(windows))]
fn write_stdout(value: &str) -> String {
    format!("printf '{value}'")
}

#[cfg(windows)]
fn write_stdout(value: &str) -> String {
    format!("[Console]::Write('{value}')")
}

#[cfg(not(windows))]
fn write_marker() -> &'static str {
    "printf spawned > marker.txt"
}

#[cfg(windows)]
fn write_marker() -> &'static str {
    "Set-Content -NoNewline marker.txt spawned"
}

#[cfg(not(windows))]
fn slow_command() -> &'static str {
    "sleep 2"
}

#[cfg(windows)]
fn slow_command() -> &'static str {
    "Start-Sleep -Seconds 2"
}

#[cfg(not(windows))]
fn read_input() -> &'static str {
    "cat input.txt"
}

#[cfg(windows)]
fn read_input() -> &'static str {
    "[Console]::Write((Get-Content -Raw input.txt))"
}

#[test]
fn terminal_tool_execution_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/terminal.rs",
        include_str!("../src/terminal.rs"),
        300,
    )
    .expect("terminal tool source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/terminal_event.rs",
        include_str!("../src/terminal_event.rs"),
        180,
    )
    .expect("terminal event source should stay below the line-count guard");
}
