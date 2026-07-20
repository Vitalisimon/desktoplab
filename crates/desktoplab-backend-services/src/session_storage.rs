use desktoplab_agent_session::{CheckpointRef, SessionEvent, TerminalEvidence};
use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use serde_json::{Value, json};

use crate::session_trace::{SessionTraceEvent, compatibility_trace};
use crate::sessions::{SessionData, SessionRecord};

pub(crate) fn load_session_data(storage: &SqliteStore) -> Result<SessionData, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::AgentSession, "sessions")?
    else {
        return Ok(SessionData::default());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::InvalidJson(error.to_string()))?;
    session_data_from_json(&value)
}

pub(crate) fn session_data_json(data: &SessionData) -> Value {
    json!({
        "nextSessionNumber":data.next_session_number,
        "records":data.records.iter().map(session_record_json).collect::<Vec<_>>()
    })
}

fn session_record_json(record: &SessionRecord) -> Value {
    json!({
        "workspaceId":record.workspace_id,
        "events":record.events.iter().map(session_event_json).collect::<Vec<_>>(),
        "trace":record.trace.iter().map(SessionTraceEvent::to_value).collect::<Vec<_>>()
        ,"turnQueue":record.turn_queue.to_value()
    })
}

fn session_event_json(event: &SessionEvent) -> Value {
    match event {
        SessionEvent::Created {
            session_id,
            backend_id,
        } => json!({"kind":"created","sessionId":session_id,"backendId":backend_id}),
        SessionEvent::PlanningStarted { plan } => json!({"kind":"planning_started","plan":plan}),
        SessionEvent::ExecutionStarted => json!({"kind":"execution_started"}),
        SessionEvent::CheckpointCreated { .. } => {
            json!({"kind":"checkpoint_created","checkpoint":"checkpoint.persisted"})
        }
        SessionEvent::Paused { reason } => json!({"kind":"paused","reason":reason}),
        SessionEvent::Resumed => json!({"kind":"resumed"}),
        SessionEvent::Blocked { reason } => json!({"kind":"blocked","reason":reason}),
        SessionEvent::BackendResponseReceived { message } => {
            json!({"kind":"backend_response_received","message":message})
        }
        SessionEvent::ToolDecisionRecorded { decision } => {
            json!({"kind":"tool_decision_recorded","decision":decision})
        }
        SessionEvent::TestCommandProposed { command } => {
            json!({"kind":"test_command_proposed","command":command})
        }
        SessionEvent::TerminalEvidenceRecorded { evidence } => json!({
            "kind":"terminal_evidence_recorded",
            "command":evidence.command(),
            "output":evidence.output(),
            "exitCode":evidence.exit_code()
        }),
        SessionEvent::JobStarted {
            job_id,
            started_at,
            cancellable,
        } => json!({
            "kind":"job_started",
            "jobId":job_id,
            "startedAt":started_at,
            "cancellable":cancellable
        }),
        SessionEvent::JobHeartbeat { job_id, at } => json!({
            "kind":"job_heartbeat",
            "jobId":job_id,
            "at":at
        }),
        SessionEvent::JobObservation { job_id, message } => json!({
            "kind":"job_observation",
            "jobId":job_id,
            "message":message
        }),
        SessionEvent::JobInterrupted {
            job_id,
            reason,
            guidance,
            at,
        } => json!({
            "kind":"job_interrupted",
            "jobId":job_id,
            "reason":reason,
            "guidance":guidance,
            "at":at
        }),
        SessionEvent::Failed { reason } => json!({"kind":"failed","reason":reason}),
        SessionEvent::Cancelled { reason } => json!({"kind":"cancelled","reason":reason}),
        SessionEvent::Completed { summary } => json!({"kind":"completed","summary":summary}),
    }
}

fn session_data_from_json(value: &Value) -> Result<SessionData, StorageError> {
    let next_session_number = value
        .get("nextSessionNumber")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let empty = Vec::new();
    let records = value
        .get("records")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
        .iter()
        .map(session_record_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SessionData {
        next_session_number,
        records,
    })
}

fn session_record_from_json(value: &Value) -> Result<SessionRecord, StorageError> {
    let workspace_id = json_string(value, "workspaceId")?;
    let events = value
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| StorageError::InvalidJson("session events missing".to_string()))?
        .iter()
        .map(session_event_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let session_id = events
        .iter()
        .find_map(|event| match event {
            SessionEvent::Created { session_id, .. } => Some(session_id.as_str()),
            _ => None,
        })
        .unwrap_or("session.legacy");
    let trace = value
        .get("trace")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(SessionTraceEvent::from_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::InvalidJson)
        })
        .transpose()?
        .unwrap_or_else(|| compatibility_trace(session_id, &events));
    Ok(SessionRecord {
        workspace_id,
        events,
        trace,
        turn_queue: crate::session_turns::SessionTurnQueue::from_value(value.get("turnQueue"))?,
    })
}

fn session_event_from_json(value: &Value) -> Result<SessionEvent, StorageError> {
    match json_string(value, "kind")?.as_str() {
        "created" => Ok(SessionEvent::created(
            json_string(value, "sessionId")?,
            json_string(value, "backendId")?,
        )),
        "planning_started" => Ok(SessionEvent::planning_started(json_string(value, "plan")?)),
        "execution_started" => Ok(SessionEvent::execution_started()),
        "checkpoint_created" => Ok(SessionEvent::checkpoint_created(CheckpointRef::new(
            json_string(value, "checkpoint")?,
        ))),
        "paused" => Ok(SessionEvent::paused(json_string(value, "reason")?)),
        "resumed" => Ok(SessionEvent::resumed()),
        "blocked" => Ok(SessionEvent::blocked(json_string(value, "reason")?)),
        "backend_response_received" => Ok(SessionEvent::backend_response_received(json_string(
            value, "message",
        )?)),
        "tool_decision_recorded" => Ok(SessionEvent::tool_decision_recorded(json_string(
            value, "decision",
        )?)),
        "test_command_proposed" => Ok(SessionEvent::test_command_proposed(json_string(
            value, "command",
        )?)),
        "terminal_evidence_recorded" => Ok(SessionEvent::terminal_evidence_recorded(
            TerminalEvidence::new(
                json_string(value, "command")?,
                json_string(value, "output")?,
                value
                    .get("exitCode")
                    .and_then(Value::as_i64)
                    .map(|code| code as i32),
            ),
        )),
        "job_started" => Ok(SessionEvent::job_started(
            json_string(value, "jobId")?,
            json_string(value, "startedAt")?,
            value
                .get("cancellable")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )),
        "job_heartbeat" => Ok(SessionEvent::job_heartbeat(
            json_string(value, "jobId")?,
            json_string(value, "at")?,
        )),
        "job_observation" => Ok(SessionEvent::job_observation(
            json_string(value, "jobId")?,
            json_string(value, "message")?,
        )),
        "job_interrupted" => Ok(SessionEvent::job_interrupted(
            json_string(value, "jobId")?,
            json_string(value, "reason")?,
            json_string(value, "guidance")?,
            json_string(value, "at")?,
        )),
        "failed" => Ok(SessionEvent::Failed {
            reason: json_string(value, "reason")?,
        }),
        "cancelled" => Ok(SessionEvent::cancelled(json_string(value, "reason")?)),
        "completed" => Ok(SessionEvent::completed(json_string(value, "summary")?)),
        other => Err(StorageError::InvalidJson(format!(
            "unknown session event kind: {other}"
        ))),
    }
}

fn json_string(value: &Value, field: &str) -> Result<String, StorageError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| StorageError::InvalidJson(format!("{field} missing")))
}
