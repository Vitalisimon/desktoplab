use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    TerminalApproval, TerminalCommandRequest, TerminalExecutionResult, TerminalExecutionStatus,
    TerminalToolExecutor, TerminalToolOutcome,
};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

const TERMINAL_OUTPUT_LIMIT: usize = 64 * 1024;

pub(crate) fn is_terminal_command_path(path: &str) -> bool {
    path.starts_with("/v1/workspaces/") && path.ends_with("/terminal/commands")
}

pub(crate) fn terminal_command_route_response(
    route_path: &str,
    workspace_id: &str,
    root_path: &str,
    body: &str,
    approval_authorized: bool,
    pending_approval_id: &str,
) -> Option<Value> {
    if route_workspace_id(route_path) != workspace_id {
        return None;
    }
    terminal_command_response(
        workspace_id,
        root_path,
        body,
        approval_authorized,
        pending_approval_id,
    )
}

pub(crate) fn terminal_command_response(
    workspace_id: &str,
    root_path: &str,
    body: &str,
    approval_authorized: bool,
    pending_approval_id: &str,
) -> Option<Value> {
    let request = TerminalCommandBody::parse(body)?;
    if !approval_authorized {
        return Some(pending_response(
            workspace_id,
            &request,
            pending_approval_id,
        ));
    }

    let mut executor = TerminalToolExecutor::new(
        Path::new(root_path),
        PolicyEngine::default_conservative(),
        Duration::from_secs(30),
        TERMINAL_OUTPUT_LIMIT,
    );
    let tool_request = TerminalCommandRequest::for_workspace(workspace_id, &request.command)
        .with_working_directory(&request.cwd);
    Some(
        match executor.execute(tool_request, TerminalApproval::Approved) {
            TerminalToolOutcome::Completed(output) => {
                completed_response(workspace_id, &request, output)
            }
            TerminalToolOutcome::Blocked(reason) => {
                blocked_response(workspace_id, &request, reason)
            }
            TerminalToolOutcome::ApprovalRequired => {
                blocked_response(workspace_id, &request, "terminal approval required")
            }
            TerminalToolOutcome::Denied => {
                blocked_response(workspace_id, &request, "terminal denied")
            }
        },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TerminalCommandBody {
    command: String,
    cwd: String,
}

impl TerminalCommandBody {
    fn parse(body: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(body).ok()?;
        Some(Self {
            command: value.get("command")?.as_str()?.to_string(),
            cwd: value
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

fn pending_response(
    workspace_id: &str,
    request: &TerminalCommandBody,
    pending_approval_id: &str,
) -> Value {
    json!({
        "terminalId":"terminal.local",
        "workspaceId":workspace_id,
        "state":"approval_required",
        "command":request.command,
        "cwd":display_cwd(&request.cwd),
        "approval":{"approvalId":pending_approval_id,"state":"pending","copy":format!("Terminal command `{}` in `{}` requires approval.", request.command, display_cwd(&request.cwd))}
    })
}

fn completed_response(
    workspace_id: &str,
    request: &TerminalCommandBody,
    output: TerminalExecutionResult,
) -> Value {
    let (status, exit_code) = match output.status() {
        TerminalExecutionStatus::Exited(code) => ("exited", Some(code)),
        TerminalExecutionStatus::TimedOut => ("timed_out", None),
        TerminalExecutionStatus::FailedToSpawn => ("failed_to_spawn", None),
    };
    json!({
        "terminalId":"terminal.local",
        "workspaceId":workspace_id,
        "state":"completed",
        "command":request.command,
        "cwd":display_cwd(&request.cwd),
        "approval":{"approvalId":"","state":"approved","copy":""},
        "events":[{
            "eventId":"terminal.local.output",
            "kind":"output",
            "status":status,
            "exitCode":exit_code,
            "stdout":output.stdout(),
            "stderr":output.stderr(),
            "stdoutTruncated":output.stdout_truncated(),
            "redacted":output.stdout().contains("[REDACTED]") || output.stderr().contains("[REDACTED]")
        }]
    })
}

fn blocked_response(workspace_id: &str, request: &TerminalCommandBody, reason: &str) -> Value {
    json!({
        "terminalId":"terminal.local",
        "workspaceId":workspace_id,
        "state":"blocked",
        "reason":reason,
        "command":request.command,
        "cwd":display_cwd(&request.cwd),
        "approval":{"approvalId":"","state":"denied","copy":format!("Terminal command blocked: {reason}.")}
    })
}

fn display_cwd(cwd: &str) -> &str {
    if cwd.is_empty() { "." } else { cwd }
}

fn route_workspace_id(path: &str) -> &str {
    path.split('/').nth(3).unwrap_or_default()
}
