use std::time::Duration;

use desktoplab_agent_engine::IterativeToolCall;
use desktoplab_tool_gateway::{
    ManagedProcessSnapshot, ManagedProcessState, TerminalApproval, TerminalCommandRequest,
    TerminalExecutionStatus, TerminalRiskClass, TerminalToolExecutor, TerminalToolOutcome,
    TestRunApproval, TestRunOutcome, TestRunRequest, TestRunnerExecutor,
};
use desktoplab_workspace::TestCommandDetector;
use serde_json::{Value, json};

use crate::canonical_tool_executor::{
    CanonicalAgentToolExecutor, CanonicalExecutionApproval, optional_string, optional_usize,
    required_string, string_argument,
};

pub(crate) fn execute(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    match call.name() {
        "desktoplab.run_terminal" => run_terminal(executor, call),
        "desktoplab.run_tests" => run_tests(executor, call),
        "desktoplab.start_process" => start_process(executor, call),
        "desktoplab.poll_process" => process_snapshot(executor.process_registry().poll(
            executor.workspace_id(),
            executor.session_id(),
            required_string(call, "processId")?,
        )?),
        "desktoplab.write_process_stdin" => {
            executor.process_registry().write_stdin(
                executor.workspace_id(),
                executor.session_id(),
                required_string(call, "processId")?,
                string_argument(call, "input")?,
            )?;
            Ok(json!({"accepted":true}))
        }
        "desktoplab.kill_process" => process_snapshot(executor.process_registry().kill(
            executor.workspace_id(),
            executor.session_id(),
            required_string(call, "processId")?,
        )?),
        _ => Err("unsupported_process_tool".to_string()),
    }
}

fn start_process(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    match executor.approval() {
        CanonicalExecutionApproval::Pending => return Err("approval_required".to_string()),
        CanonicalExecutionApproval::Denied => return Err("approval_denied".to_string()),
        CanonicalExecutionApproval::Approved => {}
    }
    let snapshot = executor.process_registry().start(
        executor.root(),
        executor.workspace_id(),
        executor.session_id(),
        required_string(call, "command")?,
        optional_string(call, "cwd").unwrap_or(""),
    )?;
    process_snapshot(snapshot)
}

fn process_snapshot(snapshot: ManagedProcessSnapshot) -> Result<Value, String> {
    let (status, exit_code) = match snapshot.state() {
        ManagedProcessState::Running => ("running", None),
        ManagedProcessState::Exited(code) => ("exited", Some(*code)),
        ManagedProcessState::Killed => ("killed", None),
    };
    Ok(json!({
        "processId":snapshot.process_id(),"status":status,"exitCode":exit_code,
        "stdout":snapshot.stdout(),"stderr":snapshot.stderr(),
        "outputTruncated":snapshot.output_truncated()
    }))
}

fn run_terminal(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    let command = required_string(call, "command")?;
    let cwd = optional_string(call, "cwd").unwrap_or("");
    let request = TerminalCommandRequest::for_workspace(executor.workspace_id(), command)
        .with_working_directory(cwd)
        .with_risk_class(TerminalRiskClass::Medium);
    let mut terminal = TerminalToolExecutor::new(
        executor.root(),
        executor.policy(),
        command_timeout(call)?,
        64 * 1024,
    );
    match terminal.execute(request, terminal_approval(executor.approval())) {
        TerminalToolOutcome::Completed(result) => {
            let (status, exit_code) = terminal_status(result.status());
            let passed = exit_code == Some(0) && is_detected_test_command(executor, command);
            Ok(json!({
                "command":command,"status":status,"exitCode":exit_code,"stdout":result.stdout(),
                "stderr":result.stderr(),"stdoutTruncated":result.stdout_truncated(),
                "stderrTruncated":result.stderr_truncated(),"passed":passed
            }))
        }
        TerminalToolOutcome::ApprovalRequired => Err("approval_required".to_string()),
        TerminalToolOutcome::Denied => Err("approval_denied".to_string()),
        TerminalToolOutcome::Blocked(reason) => Err(reason.to_string()),
    }
}

fn is_detected_test_command(executor: &CanonicalAgentToolExecutor, command: &str) -> bool {
    TestCommandDetector::detect(executor.root()).is_ok_and(|detected| {
        detected
            .commands()
            .iter()
            .any(|candidate| candidate.command() == command)
    })
}

fn run_tests(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    let command = required_string(call, "command")?;
    let request = TestRunRequest::new(
        executor.workspace_id(),
        command,
        "agent requested targeted validation",
    );
    let mut runner = TestRunnerExecutor::new(
        executor.root(),
        executor.policy(),
        command_timeout(call)?,
        64 * 1024,
    );
    match runner.run(request, test_approval(executor.approval())) {
        TestRunOutcome::Completed(evidence) => {
            let (status, exit_code) = terminal_status(evidence.status());
            Ok(json!({
                "status":status,"exitCode":exit_code,"passed":exit_code == Some(0),
                "command":evidence.command(),"stdout":evidence.stdout(),"stderr":evidence.stderr(),
                "durationMs":evidence.duration_ms(),"redactionStatus":evidence.redaction_status()
            }))
        }
        TestRunOutcome::ApprovalRequired => Err("approval_required".to_string()),
        TestRunOutcome::Denied => Err("approval_denied".to_string()),
        TestRunOutcome::Blocked(reason) => Err(reason.to_string()),
    }
}

fn command_timeout(call: &IterativeToolCall) -> Result<Duration, String> {
    optional_usize(call, "timeoutSeconds", 30, 1_800)
        .map(|seconds| Duration::from_secs(seconds as u64))
}

fn terminal_status(status: TerminalExecutionStatus) -> (&'static str, Option<i32>) {
    match status {
        TerminalExecutionStatus::Exited(code) => ("exited", Some(code)),
        TerminalExecutionStatus::TimedOut => ("timed_out", None),
        TerminalExecutionStatus::FailedToSpawn => ("failed_to_spawn", None),
    }
}

fn terminal_approval(value: CanonicalExecutionApproval) -> TerminalApproval {
    match value {
        CanonicalExecutionApproval::Pending => TerminalApproval::Pending,
        CanonicalExecutionApproval::Approved => TerminalApproval::Approved,
        CanonicalExecutionApproval::Denied => TerminalApproval::Denied,
    }
}

fn test_approval(value: CanonicalExecutionApproval) -> TestRunApproval {
    match value {
        CanonicalExecutionApproval::Pending => TestRunApproval::Pending,
        CanonicalExecutionApproval::Approved => TestRunApproval::Approved,
        CanonicalExecutionApproval::Denied => TestRunApproval::Denied,
    }
}
