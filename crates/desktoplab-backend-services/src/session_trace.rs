use std::time::{SystemTime, UNIX_EPOCH};

use desktoplab_agent_session::SessionEvent;
use desktoplab_redaction::redact_sensitive_bounded;
use serde_json::{Value, json};

use crate::session_trace_metadata::event_metadata;

const TRACE_SCHEMA_VERSION: u16 = 1;
const TRACE_DETAIL_LIMIT: usize = 256;
const TRACE_EVENT_LIMIT: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionTraceEvent {
    pub(crate) event_id: String,
    pub(crate) parent_event_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) sequence: usize,
    pub(crate) kind: String,
    pub(crate) recorded_at_unix_ms: u64,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) source: String,
    pub(crate) mutation: bool,
    pub(crate) success: Option<bool>,
    pub(crate) truncated: bool,
    pub(crate) redacted: bool,
    pub(crate) detail: String,
}

impl SessionTraceEvent {
    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    #[must_use]
    pub fn success(&self) -> Option<bool> {
        self.success
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "eventId":self.event_id,
            "parentEventId":self.parent_event_id,
            "correlationId":self.correlation_id,
            "sequence":self.sequence,
            "kind":self.kind,
            "recordedAtUnixMs":self.recorded_at_unix_ms,
            "durationMs":self.duration_ms,
            "source":self.source,
            "mutation":self.mutation,
            "success":self.success,
            "truncated":self.truncated,
            "redacted":self.redacted,
            "detail":self.detail
        })
    }

    pub(crate) fn from_value(value: &Value) -> Result<Self, String> {
        Ok(Self {
            event_id: string(value, "eventId")?,
            parent_event_id: optional_string(value, "parentEventId"),
            correlation_id: optional_string(value, "correlationId"),
            sequence: usize_value(value, "sequence")?,
            kind: string(value, "kind")?,
            recorded_at_unix_ms: u64_value(value, "recordedAtUnixMs")?,
            duration_ms: value.get("durationMs").and_then(Value::as_u64),
            source: string(value, "source")?,
            mutation: value
                .get("mutation")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            success: value.get("success").and_then(Value::as_bool),
            truncated: value
                .get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            redacted: value
                .get("redacted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            detail: string(value, "detail")?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionTraceEnvelope {
    session_id: String,
    events: Vec<SessionTraceEvent>,
}

impl SessionTraceEnvelope {
    pub(crate) fn new(session_id: impl Into<String>, events: Vec<SessionTraceEvent>) -> Self {
        Self {
            session_id: session_id.into(),
            events,
        }
    }

    #[must_use]
    pub fn schema_version(&self) -> u16 {
        TRACE_SCHEMA_VERSION
    }

    #[must_use]
    pub fn events(&self) -> &[SessionTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "schemaVersion":TRACE_SCHEMA_VERSION,
            "producer":format!("desktoplab-session-service/{}", env!("CARGO_PKG_VERSION")),
            "sessionId":self.session_id,
            "events":self.events.iter().map(SessionTraceEvent::to_value).collect::<Vec<_>>()
        })
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        self.events
            .iter()
            .map(|event| {
                serde_json::to_string(&json!({
                    "schemaVersion":TRACE_SCHEMA_VERSION,
                    "producer":format!("desktoplab-session-service/{}", env!("CARGO_PKG_VERSION")),
                    "sessionId":self.session_id,
                    "event":event.to_value()
                }))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.join("\n"))
    }
}

pub(crate) fn append_trace_event(
    session_id: &str,
    trace: &mut Vec<SessionTraceEvent>,
    event: &SessionEvent,
) {
    let sequence = trace.last().map_or(1, |event| event.sequence + 1);
    let parent_event_id = trace.last().map(|event| event.event_id.clone());
    if trace.len() >= TRACE_EVENT_LIMIT {
        trace.remove(0);
    }
    trace.push(trace_event(
        session_id,
        sequence,
        parent_event_id,
        event,
        now_unix_ms(),
    ));
}

pub(crate) fn compatibility_trace(
    session_id: &str,
    events: &[SessionEvent],
) -> Vec<SessionTraceEvent> {
    let first_index = events.len().saturating_sub(TRACE_EVENT_LIMIT);
    events
        .iter()
        .enumerate()
        .skip(first_index)
        .map(|(index, event)| {
            let sequence = index + 1;
            trace_event(
                session_id,
                sequence,
                (index > first_index).then(|| format!("{session_id}:trace:{}", sequence - 1)),
                event,
                0,
            )
        })
        .collect()
}

fn trace_event(
    session_id: &str,
    sequence: usize,
    parent_event_id: Option<String>,
    event: &SessionEvent,
    recorded_at_unix_ms: u64,
) -> SessionTraceEvent {
    let metadata = event_metadata(event);
    let bounded = redact_sensitive_bounded(&metadata.detail, TRACE_DETAIL_LIMIT);
    SessionTraceEvent {
        event_id: format!("{session_id}:trace:{sequence}"),
        parent_event_id,
        correlation_id: metadata.correlation_id,
        sequence,
        kind: metadata.kind.to_string(),
        recorded_at_unix_ms,
        duration_ms: None,
        source: metadata.source,
        mutation: metadata.mutation,
        success: metadata.success,
        truncated: metadata.detail.chars().count() > bounded.value().chars().count(),
        redacted: metadata.redacted || bounded.redacted(),
        detail: bounded.value().to_string(),
    }
}

fn string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{field} missing"))
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn usize_value(value: &Value, field: &str) -> Result<usize, String> {
    u64_value(value, field).and_then(|number| {
        number
            .try_into()
            .map_err(|_| format!("{field} out of range"))
    })
}

fn u64_value(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{field} missing"))
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}
