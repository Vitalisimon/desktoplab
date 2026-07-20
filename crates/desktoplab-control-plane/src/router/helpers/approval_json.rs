use desktoplab_backend_services::{ApprovalRequestRecord, ApprovalState};
use desktoplab_redaction::redact_sensitive;
use serde_json::{Value, json};

pub(crate) fn approval_json(approval: &ApprovalRequestRecord) -> Value {
    json!({
        "approvalId":approval.id(),
        "sessionId":approval.session_id(),
        "action":approval.action(),
        "operationId":approval.operation_id(),
        "payloadHash":approval.payload_hash(),
        "consumed":approval.is_consumed(),
        "state":approval_state_value(approval.state()),
        "risk":"medium",
        "title":approval_title(approval),
        "message":approval_message(approval),
        "requestedAt":"2026-06-26T00:00:00Z",
    })
}

fn approval_title(approval: &ApprovalRequestRecord) -> String {
    match approval.action() {
        "filesystem.write" if is_filesystem_patch(approval) => {
            format!("Patch {}", approval_target(approval))
        }
        "filesystem.write" => format!("Write {}", approval_target(approval)),
        "terminal.command" => format!("Run {}", approval_target(approval)),
        "test.run" => format!("Run {}", approval_target(approval)),
        "git.commit" => "Commit changes".to_string(),
        "git.push" => "Push changes".to_string(),
        "provider.egress" => "Send context to provider".to_string(),
        action => action.to_string(),
    }
}

fn approval_message(approval: &ApprovalRequestRecord) -> String {
    match approval.action() {
        "filesystem.write" if is_filesystem_patch(approval) => format!(
            "DesktopLab wants to update {} with a localized patch.",
            approval_target(approval)
        ),
        "filesystem.write" => format!(
            "DesktopLab wants to create or edit {}.",
            approval_target(approval)
        ),
        "terminal.command" => format!(
            "DesktopLab wants to run `{}` in the workspace terminal.",
            approval_target(approval)
        ),
        "test.run" => format!(
            "DesktopLab wants to run validation command `{}` in the workspace.",
            approval_target(approval)
        ),
        "git.commit" => "DesktopLab wants to create a Git commit.".to_string(),
        "git.push" => "DesktopLab wants to push to a remote repository.".to_string(),
        "provider.egress" => {
            "DesktopLab wants to send repository context outside this computer.".to_string()
        }
        action => format!("DesktopLab requests approval for {action}."),
    }
}

fn is_filesystem_patch(approval: &ApprovalRequestRecord) -> bool {
    approval.operation_id().starts_with("filesystem.patch:")
}

fn approval_target(approval: &ApprovalRequestRecord) -> String {
    redact_sensitive(
        &approval
            .operation_id()
            .split_once(':')
            .map(|(_, target)| target)
            .unwrap_or_else(|| approval.operation_id())
            .to_string(),
    )
}

pub(crate) fn approval_state_value(state: ApprovalState) -> &'static str {
    match state {
        ApprovalState::Pending => "pending",
        ApprovalState::Approved => "approved",
        ApprovalState::Denied => "denied",
        ApprovalState::Expired => "expired",
    }
}
