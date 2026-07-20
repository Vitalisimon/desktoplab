use desktoplab_tool_gateway::TerminalCommandRequest;

use crate::{ApprovalRequestRecord, ApprovalState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalCommandApproval {
    approval_id: String,
    state: ApprovalState,
    command: String,
    cwd: String,
    copy: String,
    evidence: String,
}

impl TerminalCommandApproval {
    #[must_use]
    pub fn approval_id(&self) -> &str {
        &self.approval_id
    }

    #[must_use]
    pub fn state(&self) -> ApprovalState {
        self.state
    }

    #[must_use]
    pub fn can_run(&self) -> bool {
        self.state == ApprovalState::Approved
    }

    #[must_use]
    pub fn copy(&self) -> &str {
        &self.copy
    }

    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }
}

pub(crate) fn terminal_approval(
    record: &ApprovalRequestRecord,
    request: &TerminalCommandRequest,
) -> TerminalCommandApproval {
    let command = request.command().to_string();
    let cwd = display_cwd(request);
    let state_copy = match record.state() {
        ApprovalState::Pending => "requires approval",
        ApprovalState::Approved => "approved",
        ApprovalState::Denied => "denied",
        ApprovalState::Expired => "expired",
    };
    TerminalCommandApproval {
        approval_id: record.id().to_string(),
        state: record.state(),
        copy: format!("Terminal command `{command}` in `{cwd}` {state_copy}."),
        evidence: format!(
            "terminal command {state_copy}: {} in `{cwd}`",
            redact_terminal_command(&command)
        ),
        command,
        cwd,
    }
}

fn display_cwd(request: &TerminalCommandRequest) -> String {
    let cwd = request.working_directory();
    if cwd.as_os_str().is_empty() {
        ".".to_string()
    } else {
        cwd.to_string_lossy().to_string()
    }
}

fn redact_terminal_command(command: &str) -> String {
    command
        .split_whitespace()
        .map(|part| {
            if let Some((key, _)) = part.split_once('=')
                && matches!(key.to_ascii_lowercase().as_str(), "token" | "secret")
            {
                return format!("{key}=<redacted>:");
            }
            part.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
