use desktoplab_backend_services::{BackendEventFrame, BackendEventScope, JobState};
use serde_json::{Value, json};

use super::payload_hash::stable_payload_hash;

use super::WorkspaceRecord;

mod approval_json;
mod query;
mod setup_selection;
pub(crate) use approval_json::approval_json;
pub(crate) use query::query_value;
pub(crate) use setup_selection::{
    setup_accept_selection, valid_model_for_runtime, valid_runtime_id,
};
pub(crate) fn workspace_json(workspace: &WorkspaceRecord) -> Value {
    let git = git_snapshot(&workspace.root_path);
    let root_exists = std::path::Path::new(&workspace.root_path).is_dir();
    json!({
        "workspaceId":workspace.workspace_id,
        "displayName":workspace.display_name,
        "rootPath":workspace.root_path,
        "rootExists":root_exists,
        "stale":!root_exists,
        "readOnly":!root_exists,
        "blockedReason":if root_exists { Value::Null } else { json!("workspace_root_missing") },
        "gitDirPath":git.git_dir_path,
        "apiState":git.api_state,
        "statusEntries":git.status_entries,
        "diffText":git.diff_text,
        "checkpointStatus":git.checkpoint_status,
        "canCheckpointRiskyExecution":git.can_checkpoint_risky_execution
    })
}

struct WorkspaceGitSnapshot {
    git_dir_path: String,
    api_state: &'static str,
    status_entries: Vec<String>,
    diff_text: String,
    checkpoint_status: &'static str,
    can_checkpoint_risky_execution: bool,
}

fn git_snapshot(root_path: &str) -> WorkspaceGitSnapshot {
    let fallback = || WorkspaceGitSnapshot {
        git_dir_path: format!("{root_path}/.git"),
        api_state: "clean",
        status_entries: Vec::new(),
        diff_text: String::new(),
        checkpoint_status: "ready",
        can_checkpoint_risky_execution: true,
    };
    let Ok(repo) = desktoplab_workspace::GitRepository::open(std::path::Path::new(root_path))
    else {
        return fallback();
    };
    let status = repo.status().ok();
    let dirty = status.as_ref().is_some_and(|status| status.is_dirty());
    WorkspaceGitSnapshot {
        git_dir_path: repo.identity().git_dir_path().display().to_string(),
        api_state: if dirty { "dirty" } else { "clean" },
        status_entries: status
            .map(|status| status.entries().to_vec())
            .unwrap_or_default(),
        diff_text: repo
            .diff()
            .map(|diff| diff.as_text().to_string())
            .unwrap_or_default(),
        checkpoint_status: "ready",
        can_checkpoint_risky_execution: true,
    }
}

pub(crate) fn event_frame_json(frame: &BackendEventFrame) -> Value {
    json!({
        "sequence":frame.sequence(),
        "scope":frame.scope().map(event_scope_value),
        "payload":frame.payload()
    })
}

pub(crate) fn event_scope_value(scope: BackendEventScope) -> &'static str {
    match scope {
        BackendEventScope::Job => "job",
        BackendEventScope::Session => "session",
        BackendEventScope::Approval => "approval",
        BackendEventScope::Setup => "setup",
        BackendEventScope::Terminal => "terminal",
    }
}

pub(crate) fn job_state_value(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::AwaitingApproval => "blocked",
        JobState::Succeeded => "completed",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::Blocked => "blocked",
    }
}

pub(crate) fn string_field(value: &Value, field: &str, default: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

pub(crate) fn plugin_id_from_trust_path(path: &str) -> &str {
    path.trim_start_matches("/v1/plugins/")
        .trim_end_matches("/trust")
        .trim_matches('/')
}

pub(crate) fn terminal_operation_id(workspace_id: &str) -> String {
    format!("{workspace_id}:terminal.local")
}

pub(crate) fn path_without_query(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

pub(crate) fn body_field(body: &str, field: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value.get(field)?.as_str().map(ToString::to_string)
}

pub(crate) fn body_field_or(body: &str, field: &str, default: &str) -> String {
    body_field(body, field).unwrap_or_else(|| default.to_string())
}

pub(crate) fn body_bool_or(body: &str, field: &str, default: bool) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return default;
    };
    value.get(field).and_then(Value::as_bool).unwrap_or(default)
}

pub(crate) fn approval_payload_hash_from_payload(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value.get("payload").map(stable_payload_hash)
}

pub(crate) fn terminal_command_payload_hash(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let payload = json!({
        "command":value.get("command")?.as_str()?,
        "cwd":value.get("cwd").and_then(Value::as_str).unwrap_or_default(),
    });
    Some(stable_payload_hash(&payload))
}

pub(crate) fn git_commit_payload_hash(body: &str) -> String {
    let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    stable_payload_hash(&json!({
        "changeFingerprint":value.get("changeFingerprint").and_then(Value::as_str).unwrap_or(""),
        "changedFiles":value.get("changedFiles").and_then(Value::as_array).cloned().unwrap_or_default(),
        "message":value.get("message").and_then(Value::as_str).unwrap_or("agent change"),
        "sessionId":value.get("sessionId").and_then(Value::as_str).unwrap_or("session.local"),
    }))
}

pub(crate) fn body_string_array(body: &str, field: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn segment(path: &str, index: usize) -> String {
    path.split('/')
        .nth(index + 1)
        .unwrap_or_default()
        .to_string()
}
