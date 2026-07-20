use desktoplab_agent_session::{AgentSession, SessionEvent, SessionState};
use desktoplab_redaction::redact_sensitive_with_status;
use serde_json::{Value, json};

pub(crate) fn display_turns(session: &AgentSession) -> Vec<Value> {
    let completed = session.state() == SessionState::Completed;
    session
        .event_log()
        .iter()
        .enumerate()
        .filter_map(|(index, event)| display_turn(index + 1, event, completed))
        .collect()
}

pub(crate) fn context_transcript_with_compaction(
    session: &AgentSession,
    compaction: Option<&super::agent_compaction::AgentContextCompaction>,
    max_turns: usize,
) -> Option<String> {
    let mut turns = indexed_context_turns(session);
    if let Some(compaction) = compaction {
        turns.retain(|(sequence, _)| *sequence > compaction.through_event_sequence());
    }
    let keep_from = turns.len().saturating_sub(max_turns);
    let selected = turns
        .drain(keep_from..)
        .map(|(_, turn)| turn)
        .collect::<Vec<_>>();
    let mut sections = Vec::new();
    if let Some(compaction) = compaction {
        sections.push(format!(
            "compacted_prior_transcript:\n{}",
            compaction.summary()
        ));
    }
    if !selected.is_empty() {
        sections.push(format!("recent_transcript:\n{}", selected.join("\n")));
    }
    (!sections.is_empty()).then(|| sections.join("\n"))
}

pub(super) fn indexed_context_turns(session: &AgentSession) -> Vec<(usize, String)> {
    session
        .event_log()
        .iter()
        .enumerate()
        .filter_map(|(index, event)| context_turn(event).map(|turn| (index + 1, turn)))
        .collect()
}

pub(crate) fn details(session: &AgentSession) -> Value {
    let mut tool_calls = Vec::new();
    let mut approvals = Vec::new();
    let mut observations = Vec::new();
    let mut diffs = Vec::new();
    let mut validations = Vec::new();

    for event in session.event_log() {
        match event {
            SessionEvent::ToolDecisionRecorded { decision } => {
                if let Some(item) = tool_detail(decision) {
                    if decision.contains("approval_required") {
                        approvals.push(item.clone());
                    }
                    tool_calls.push(item);
                }
            }
            SessionEvent::BackendResponseReceived { message }
                if message.starts_with("Git diff:") =>
            {
                let item = redacted_detail("diff", message, "agent.git_diff");
                diffs.push(item.clone());
                observations.push(item);
            }
            SessionEvent::BackendResponseReceived { message }
                if message.starts_with("Test command `") =>
            {
                let item = redacted_detail("validation", message, "agent.validation");
                validations.push(item.clone());
                observations.push(item);
            }
            SessionEvent::BackendResponseReceived { message }
                if internal_executor_observation(message) =>
            {
                observations.push(redacted_detail("observation", message, "agent.observation"));
            }
            SessionEvent::TerminalEvidenceRecorded { evidence } => {
                if !evidence.output().starts_with("Test command `") {
                    observations.push(redacted_detail(
                        "terminal",
                        evidence.output(),
                        "agent.terminal",
                    ));
                }
            }
            _ => {}
        }
    }

    json!({
        "plan":session.plan(),
        "toolCalls":tool_calls,
        "approvals":approvals,
        "observations":observations,
        "diffs":diffs,
        "validations":validations
    })
}

fn display_turn(sequence: usize, event: &SessionEvent, completed: bool) -> Option<Value> {
    match event {
        SessionEvent::PlanningStarted { plan } => Some(turn(sequence, "user", visible_text(plan))),
        SessionEvent::BackendResponseReceived { message }
            if internal_executor_observation(message) =>
        {
            None
        }
        SessionEvent::BackendResponseReceived { message } => {
            Some(turn(sequence, "assistant", visible_text(message)))
        }
        SessionEvent::ToolDecisionRecorded { decision } if !completed => {
            Some(turn(sequence, "tool", tool_label(decision)))
        }
        SessionEvent::Blocked { .. } if completed => None,
        SessionEvent::Blocked { reason } => Some(turn(sequence, "status", reason.to_string())),
        SessionEvent::Failed { reason } => Some(turn(sequence, "status", reason.to_string())),
        SessionEvent::TerminalEvidenceRecorded { evidence }
            if !completed && !evidence.output().starts_with("Test command `") =>
        {
            Some(turn(sequence, "tool", evidence.output().to_string()))
        }
        _ => None,
    }
}

fn context_turn(event: &SessionEvent) -> Option<String> {
    let (role, content) = match event {
        SessionEvent::PlanningStarted { plan } => ("user", visible_text(plan)),
        SessionEvent::BackendResponseReceived { message } => ("assistant", visible_text(message)),
        SessionEvent::Blocked { reason } => {
            let question = reason.strip_prefix("clarification_required:")?;
            ("assistant", question.to_string())
        }
        _ => return None,
    };
    let content = content.trim();
    if content.is_empty() || internal_context_noise(content) {
        return None;
    }
    let redacted = redact_sensitive_with_status(content);
    Some(format!("{role}: {}", redacted.value()))
}

fn internal_context_noise(message: &str) -> bool {
    internal_executor_observation(message)
}

fn internal_executor_observation(message: &str) -> bool {
    message.starts_with("Observation: tool ")
        || message.starts_with("Test command `")
        || message.starts_with("Git diff")
        || message.starts_with("Git status:")
        || (message.starts_with("Read ") && message.contains(":\n"))
        || message.starts_with("Changed ")
        || message.starts_with("Command `")
        || message.starts_with("Workspace files:\n")
        || ((message.starts_with("Search results for ") || message.starts_with("Search results:"))
            && message.contains(":\n"))
        || message.starts_with("Checkpoint ready:")
        || message.starts_with("Git commit created:")
        || message.starts_with("Git push completed:")
}

fn turn(sequence: usize, role: &str, content: String) -> Value {
    let redacted = redact_sensitive_with_status(&content);
    json!({
        "sequence":sequence,
        "role":role,
        "content":redacted.value(),
        "redacted":redacted.redacted()
    })
}

fn redacted_detail(kind: &str, message: &str, source: &str) -> Value {
    let redacted = redact_sensitive_with_status(message);
    json!({
        "kind":kind,
        "message":redacted.value(),
        "redacted":redacted.redacted(),
        "redactionSource":source
    })
}

fn visible_text(message: &str) -> String {
    if let Some(message) = structured_assistant_message(message) {
        return message;
    }
    if provider_tool_call_message(message) {
        return String::new();
    }
    message.to_string()
}

fn structured_assistant_message(message: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(message).ok()?;
    value
        .get("assistantMessage")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn provider_tool_call_message(message: &str) -> bool {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return false;
    }
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    let mut count = 0;
    for value in stream {
        let Ok(value) = value else {
            return false;
        };
        if !provider_tool_call_value(&value) {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn provider_tool_call_value(value: &Value) -> bool {
    value
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.trim().is_empty())
        && value.get("arguments").is_some()
}

fn tool_detail(decision: &str) -> Option<Value> {
    let state = field_value(decision, "state")?.trim();
    let tool = field_value(decision, "tool")?.trim();
    if state.is_empty() || tool.is_empty() {
        return None;
    }
    Some(json!({
        "state":state,
        "source":field_value(decision, "source").unwrap_or_default(),
        "tool":redacted_text(tool),
        "approvalMode":field_value(decision, "approval_mode").unwrap_or_default()
    }))
}

fn tool_label(decision: &str) -> String {
    let state = field_value(decision, "state").unwrap_or("tool");
    let tool = field_value(decision, "tool").unwrap_or("unknown");
    format!("{state} · {}", redacted_text(tool))
}

fn redacted_text(value: &str) -> String {
    redact_sensitive_with_status(value).value().to_string()
}

fn field_value<'a>(message: &'a str, key: &str) -> Option<&'a str> {
    message
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_session::{AgentSession, SessionEvent};

    use super::display_turns;

    #[test]
    fn executor_observations_never_reenter_the_conversation_transcript() {
        let mut session = AgentSession::new("session.1", "backend.ollama");
        session.apply(SessionEvent::planning_started("Inspect the repository"));
        for observation in [
            "Workspace files:\nREADME.md",
            "Read README.md:\n# Fixture",
            "Search results for `module`:\nREADME.md: module",
            "Git status: redacted=false redaction_source=git.status\n?? README.md",
            "Observation: tool filesystem.read:missing.md failed safely with executor_reason=read_failed.",
            "Checkpoint ready: desktoplab/savepoints/checkpoint.1",
            "Git commit created: docs: fixture",
        ] {
            session.apply(SessionEvent::backend_response_received(observation));
        }
        session.apply(SessionEvent::backend_response_received(
            "Repository inspection complete.",
        ));
        session.apply(SessionEvent::completed("agent loop completed"));
        session.apply(SessionEvent::planning_started("Run another action"));

        let transcript = serde_json::to_string(&display_turns(&session)).unwrap();

        assert!(transcript.contains("Repository inspection complete."));
        assert!(!transcript.contains("Workspace files:"), "{transcript}");
        assert!(!transcript.contains("Read README.md:"), "{transcript}");
        assert!(!transcript.contains("Search results for"), "{transcript}");
        assert!(!transcript.contains("redaction_source"), "{transcript}");
        assert!(!transcript.contains("executor_reason"), "{transcript}");
        assert!(!transcript.contains("Checkpoint ready:"), "{transcript}");
        assert!(!transcript.contains("Git commit created:"), "{transcript}");
    }
}
