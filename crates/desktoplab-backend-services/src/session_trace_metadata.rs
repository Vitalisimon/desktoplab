use desktoplab_agent_session::SessionEvent;
use desktoplab_tool_gateway::{canonical_tool_from_record, canonical_tool_mutates};

pub(crate) struct EventMetadata {
    pub(crate) kind: &'static str,
    pub(crate) source: String,
    pub(crate) mutation: bool,
    pub(crate) success: Option<bool>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) redacted: bool,
    pub(crate) detail: String,
}

pub(crate) fn event_metadata(event: &SessionEvent) -> EventMetadata {
    let (kind, source, mutation, success, correlation_id, redacted, detail) = match event {
        SessionEvent::Created { backend_id, .. } => (
            "session_created",
            safe_token(backend_id, "backend"),
            false,
            None,
            None,
            false,
            "session_record_created".to_string(),
        ),
        SessionEvent::PlanningStarted { .. } => (
            "prompt_recorded",
            "user".to_string(),
            false,
            None,
            None,
            true,
            "user_prompt_recorded".to_string(),
        ),
        SessionEvent::ExecutionStarted => simple("execution_started", "agent"),
        SessionEvent::CheckpointCreated { .. } => (
            "checkpoint_recorded",
            "workspace".to_string(),
            true,
            Some(true),
            None,
            true,
            "checkpoint_reference_recorded".to_string(),
        ),
        SessionEvent::Paused { .. } => private("paused", "agent", "pause_reason_recorded"),
        SessionEvent::Resumed => simple("resumed", "agent"),
        SessionEvent::Blocked { .. } => private("blocked", "policy", "block_reason_recorded"),
        SessionEvent::BackendResponseReceived { .. } => (
            "model_response_recorded",
            "model".to_string(),
            false,
            None,
            None,
            true,
            "model_response_recorded".to_string(),
        ),
        SessionEvent::ToolDecisionRecorded { decision } => tool_metadata(decision),
        SessionEvent::TestCommandProposed { .. } => (
            "terminal_proposed",
            "terminal".to_string(),
            true,
            None,
            None,
            true,
            "terminal_command_withheld".to_string(),
        ),
        SessionEvent::TerminalEvidenceRecorded { evidence } => (
            "terminal_observed",
            "terminal".to_string(),
            false,
            evidence.exit_code().map(|code| code == 0),
            None,
            true,
            format!(
                "exit_code={}",
                evidence
                    .exit_code()
                    .map_or_else(|| "none".to_string(), |code| code.to_string())
            ),
        ),
        SessionEvent::JobStarted { job_id, .. } => job("job_started", job_id, None),
        SessionEvent::JobHeartbeat { job_id, .. } => job("job_heartbeat", job_id, None),
        SessionEvent::JobObservation { job_id, .. } => job("job_observed", job_id, None),
        SessionEvent::JobInterrupted { job_id, .. } => job("job_interrupted", job_id, Some(false)),
        SessionEvent::Failed { .. } => terminal("failed", false),
        SessionEvent::Cancelled { .. } => terminal("cancelled", false),
        SessionEvent::Completed { .. } => terminal("completed", true),
    };
    EventMetadata {
        kind,
        source,
        mutation,
        success,
        correlation_id,
        redacted,
        detail,
    }
}

type MetadataParts = (
    &'static str,
    String,
    bool,
    Option<bool>,
    Option<String>,
    bool,
    String,
);

fn simple(kind: &'static str, source: &str) -> MetadataParts {
    (
        kind,
        source.to_string(),
        false,
        None,
        None,
        false,
        kind.to_string(),
    )
}

fn private(kind: &'static str, source: &str, detail: &str) -> MetadataParts {
    (
        kind,
        source.to_string(),
        false,
        None,
        None,
        true,
        detail.to_string(),
    )
}

fn terminal(kind: &'static str, success: bool) -> MetadataParts {
    (
        kind,
        "agent".to_string(),
        false,
        Some(success),
        None,
        true,
        format!("{kind}_summary_recorded"),
    )
}

fn job(kind: &'static str, job_id: &str, success: Option<bool>) -> MetadataParts {
    let correlation = safe_token(job_id, "job");
    (
        kind,
        "job".to_string(),
        false,
        success,
        Some(correlation.clone()),
        true,
        format!("job_id={correlation}"),
    )
}

fn tool_metadata(decision: &str) -> MetadataParts {
    let state = field(decision, "state")
        .or_else(|| field(decision, "event"))
        .unwrap_or("recorded");
    let source = field(decision, "source").unwrap_or("tool");
    let evidence = field(decision, "tool").unwrap_or_default();
    let tool = field(decision, "canonical")
        .and_then(|id| canonical_tool_from_record("agent.iterative", id))
        .or_else(|| canonical_tool_from_record(source, evidence))
        .unwrap_or_else(|| "tool".to_string());
    let approval_state = field(decision, "approval_state");
    let kind = match (state, approval_state) {
        ("approval_required", _) | ("before_tool", Some("pending")) => "approval_required",
        ("approved" | "denied" | "rejected", _) => "approval_resolved",
        ("executed" | "observed" | "failed", _) => "tool_observed",
        ("proposed" | "planned" | "before_tool", _) => "tool_requested",
        _ => "tool_decision_recorded",
    };
    let success = match state {
        "approved" | "executed" | "observed" => Some(true),
        "denied" | "rejected" | "failed" => Some(false),
        _ => None,
    };
    let mutation = field(decision, "mutation")
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or_else(|| {
            matches!(state, "executed" | "observed" | "failed") && canonical_tool_mutates(&tool)
        });
    (
        kind,
        tool.clone(),
        mutation,
        success,
        None,
        true,
        format!("state={} tool={tool}", safe_token(state, "recorded")),
    )
}

fn field<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    value
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
}

fn safe_token(value: &str, fallback: &str) -> String {
    let safe = value.chars().count() <= 96
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if safe && !value.is_empty() {
        value.to_string()
    } else {
        fallback.to_string()
    }
}
