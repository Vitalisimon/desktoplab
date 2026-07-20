use desktoplab_agent_session::{AgentJobSnapshot, AgentSession, SessionEvent, SessionState};
use desktoplab_backend_services::SessionTraceEnvelope;
use desktoplab_redaction::redact_sensitive_with_status;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent_failure::session_failure_payload;
use crate::router::agent_transcript;

pub(crate) fn context(workspace_id: String) -> Value {
    json!({"workspaceId":workspace_id,"languages":["TypeScript","Rust"],"frameworks":["React","Tauri"],"testCommands":[{"command":"npm --prefix apps/desktop run check","confidence":"confirmed"}],"protectedSummary":[".env and credential files are excluded"],"stale":false,"refreshSupported":false})
}

pub(crate) fn git_operations(workspace_id: String) -> Value {
    json!({"workspaceId":workspace_id,"workspaceState":"clean","warnings":[],"changedFiles":[],"statusEntries":[],"diffPreview":"","savePoints":[],"commit":{"supported":false,"sessionId":"session.unavailable","message":"agent change","preview":"No completed change to commit.","requiresApproval":true},"push":{"supported":false,"remote":"origin","branch":"main","preview":"Push requires approval and a committed change.","requiresApproval":true,"normalizedReason":"no_commit"},"worktrees":[]})
}

pub(crate) fn workspace_intelligence(
    workspace_id: String,
    display_name: String,
    root_path: String,
) -> Value {
    json!({"workspaceId":workspace_id,"projectType":"desktop-app","stale":false,"refreshSupported":false,"facts":[{"label":"Project","value":display_name,"confidence":"confirmed"},{"label":"Path","value":root_path,"confidence":"confirmed"},{"label":"Stack","value":"Rust, Tauri, React","confidence":"confirmed"}],"testCommands":[{"command":"npm --prefix apps/desktop run check","confidence":"confirmed"}],"protectedSummary":[".env, SSH keys and credential files excluded"],"diagnosticsLink":"diagnostics"})
}

pub(crate) fn context_preview(workspace_id: String) -> Value {
    json!({"workspaceId":workspace_id,"included":["source files","tests","docs"],"excluded":[".git","secrets",".env","SSH keys"],"estimatedTokens":4200,"requiresApprovalForProviderEgress":true})
}

pub(crate) fn session(session: Option<&AgentSession>, workspace_id: &str) -> Value {
    let (session_id, backend_id, state) = session
        .map(|session| {
            (
                session.session_id().to_string(),
                session.execution_backend_id().to_string(),
                session_state(session.state()),
            )
        })
        .unwrap_or_else(|| {
            (
                "session.unavailable".to_string(),
                "backend.unavailable".to_string(),
                "blocked",
            )
        });
    let plan = session
        .and_then(AgentSession::plan)
        .unwrap_or("Inspect, edit, test.");
    let timeline = session_timeline(session);
    let transcript = session
        .map(agent_transcript::display_turns)
        .unwrap_or_default();
    let details = session
        .map(agent_transcript::details)
        .unwrap_or_else(|| json!({"plan":null,"toolCalls":[],"approvals":[],"observations":[],"diffs":[],"validations":[]}));
    let summary = session.and_then(AgentSession::summary).map(redacted_text);
    let blocked_reason = session.and_then(AgentSession::blocked_reason);
    let job = session.and_then(AgentSession::job).map(job_payload);
    let job_cancellable = session
        .and_then(AgentSession::job)
        .is_some_and(AgentJobSnapshot::cancellable);
    let failure_classification = session.map(session_failure_payload).unwrap_or(Value::Null);
    json!({"sessionId":session_id,"workspaceId":workspace_id,"executionBackendId":backend_id,"owner":"desktoplab","state":state,"blockedReason":blocked_reason.map(redacted_text),"plan":redacted_text(plan),"checkpoints":[],"summary":summary,"controls":{"pause":state=="running","resume":state=="paused"||state=="blocked","cancel":(state=="running"&&job_cancellable)||state=="blocked"},"timeline":timeline,"transcript":transcript,"details":details,"job":job,"failureClassification":failure_classification})
}

pub(crate) fn session_with_pending_approvals(
    session: Option<&AgentSession>,
    workspace_id: &str,
    pending_approvals: Vec<Value>,
    trace: Option<&SessionTraceEnvelope>,
) -> Value {
    let mut payload = self::session(session, workspace_id);
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "pendingApprovals".to_string(),
            Value::Array(pending_approvals),
        );
        object.insert(
            "trace".to_string(),
            trace
                .map(SessionTraceEnvelope::to_value)
                .unwrap_or(Value::Null),
        );
    }
    payload
}

fn session_timeline(session: Option<&AgentSession>) -> Vec<Value> {
    let Some(session) = session else {
        return Vec::new();
    };
    session
        .event_log()
        .iter()
        .filter_map(timeline_event)
        .enumerate()
        .map(|(index, (kind, message))| timeline_row(index + 1, kind, message))
        .collect()
}

fn timeline_event(event: &SessionEvent) -> Option<(&'static str, &str)> {
    match event {
        SessionEvent::PlanningStarted { plan } => Some(("planning", plan)),
        SessionEvent::BackendResponseReceived { message } => Some(("assistant", message)),
        SessionEvent::ToolDecisionRecorded { decision } => Some(("tool_decision", decision)),
        SessionEvent::Blocked { reason } => Some(("blocked", reason)),
        SessionEvent::Failed { reason } => Some(("failed", reason)),
        SessionEvent::TestCommandProposed { command } => Some(("test", command)),
        SessionEvent::TerminalEvidenceRecorded { evidence } => Some(("tool", evidence.output())),
        SessionEvent::JobStarted { job_id, .. } => Some(("job", job_id)),
        SessionEvent::JobHeartbeat { at, .. } => Some(("job", at)),
        SessionEvent::JobObservation { message, .. } => Some(("job", message)),
        SessionEvent::JobInterrupted { guidance, .. } => Some(("job", guidance)),
        SessionEvent::Created { .. }
        | SessionEvent::ExecutionStarted
        | SessionEvent::CheckpointCreated { .. }
        | SessionEvent::Paused { .. }
        | SessionEvent::Resumed
        | SessionEvent::Cancelled { .. }
        | SessionEvent::Completed { .. } => None,
    }
}

fn timeline_row(sequence: usize, kind: &str, message: &str) -> Value {
    let redacted = redact_sensitive_with_status(message);
    json!({
        "sequence":sequence,
        "kind":kind,
        "message":redacted.value(),
        "redacted":redacted.redacted(),
        "createdAt":current_timestamp()
    })
}

fn job_payload(job: &AgentJobSnapshot) -> Value {
    json!({
        "jobId":job.job_id(),
        "state":job.state(),
        "startedAt":job.started_at(),
        "lastHeartbeatAt":job.last_heartbeat_at(),
        "lastObservation":job.last_observation().map(redacted_text),
        "cancellable":job.cancellable(),
        "recoveryGuidance":job.recovery_guidance().map(redacted_text)
    })
}

fn redacted_text(value: &str) -> String {
    redact_sensitive_with_status(value).value().to_string()
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn session_state(state: SessionState) -> &'static str {
    match state {
        SessionState::Created => "created",
        SessionState::Planning => "planning",
        SessionState::Running => "running",
        SessionState::Paused => "paused",
        SessionState::Blocked => "blocked",
        SessionState::Failed => "failed",
        SessionState::Cancelled => "cancelled",
        SessionState::Completed => "completed",
    }
}
