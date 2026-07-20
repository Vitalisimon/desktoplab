use desktoplab_agent_engine::IterativeToolCall;
use desktoplab_tool_gateway::{GitToolExecutor, GitToolOutcome};
use desktoplab_workspace::{
    CommitApproval, CommitOperation, GitRepository, PushApproval, PushOperation, SavePointManager,
};
use serde_json::{Value, json};

use crate::canonical_tool_executor::{
    CanonicalAgentToolExecutor, CanonicalExecutionApproval, optional_string, required_string,
};

pub(crate) fn execute(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    match call.name() {
        "desktoplab.git_status" => status(executor),
        "desktoplab.git_diff" => diff(executor, optional_string(call, "path")),
        "desktoplab.create_checkpoint" => checkpoint(executor, required_string(call, "label")?),
        "desktoplab.commit_changes" => commit(executor, call),
        "desktoplab.push_changes" => push(
            executor,
            required_string(call, "remote")?,
            required_string(call, "branch")?,
        ),
        _ => Err("unsupported_git_tool".to_string()),
    }
}

fn status(executor: &CanonicalAgentToolExecutor) -> Result<Value, String> {
    let mut git = GitToolExecutor::new(executor.root(), executor.policy());
    match git.status() {
        GitToolOutcome::Status(status) => Ok(json!({"entries":status.entries()})),
        GitToolOutcome::Blocked(reason) => Err(reason.to_string()),
        _ => Err("git_status_unavailable".to_string()),
    }
}

fn diff(executor: &CanonicalAgentToolExecutor, path: Option<&str>) -> Result<Value, String> {
    let mut git = GitToolExecutor::new(executor.root(), executor.policy());
    let text = match path {
        Some(path) => git.diff_path_observation(path),
        None => git.diff_observation(),
    }
    .map_err(ToString::to_string)?;
    Ok(json!({"path":path,"diff":text}))
}

fn checkpoint(executor: &CanonicalAgentToolExecutor, label: &str) -> Result<Value, String> {
    let id = format!(
        "{}-{}",
        safe_ref_fragment(executor.session_id()),
        safe_ref_fragment(label)
    );
    let savepoint = SavePointManager::default()
        .create(executor.root(), &id)
        .map_err(|error| error.to_string())?;
    Ok(json!({"status":"created","ref":savepoint.ref_name()}))
}

fn commit(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    let message = required_string(call, "message")?;
    let repo = GitRepository::open(executor.root()).map_err(|error| error.to_string())?;
    let status = repo.status().map_err(|error| error.to_string())?;
    let files = status
        .files()
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();
    let requested = call
        .arguments()
        .get("paths")
        .and_then(Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .map(|path| {
                    path.as_str()
                        .filter(|path| !path.trim().is_empty())
                        .map(ToString::to_string)
                        .ok_or_else(|| "invalid_argument:paths".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut selected = if requested.is_empty() {
        files
    } else if requested.iter().all(|path| files.contains(path)) {
        requested
    } else {
        return Err("selected_commit_path_not_changed".to_string());
    };
    selected.sort();
    selected.dedup();
    require_approved(executor.approval())?;
    let outcome = CommitOperation::new(CommitApproval::Approved)
        .commit(executor.root(), executor.session_id(), message, &selected)
        .map_err(|error| error.to_string())?;
    Ok(json!({"status":outcome.status(),"message":message,"files":selected}))
}

fn push(
    executor: &CanonicalAgentToolExecutor,
    remote: &str,
    branch: &str,
) -> Result<Value, String> {
    require_approved(executor.approval())?;
    let outcome = PushOperation::new(PushApproval::Approved)
        .push(executor.root(), remote, branch)
        .map_err(|error| error.to_string())?;
    Ok(json!({"status":outcome.status(),"remote":remote,"branch":branch}))
}

fn require_approved(approval: CanonicalExecutionApproval) -> Result<(), String> {
    match approval {
        CanonicalExecutionApproval::Approved => Ok(()),
        CanonicalExecutionApproval::Pending => Err("approval_required".to_string()),
        CanonicalExecutionApproval::Denied => Err("approval_denied".to_string()),
    }
}

fn safe_ref_fragment(value: &str) -> String {
    let fragment = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    fragment.trim_matches('-').chars().take(80).collect()
}
