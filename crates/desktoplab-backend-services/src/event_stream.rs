use desktoplab_redaction::redact_sensitive_with_status;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

mod payload;
mod progress;
mod replay;

use payload::BackendEventPayload;
pub use replay::{EventReplayRequest, EventReplayResponse};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendEventScope {
    Job,
    Session,
    Approval,
    Setup,
    Terminal,
}

const DEFAULT_EVENT_LIMIT: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendEventFrame {
    sequence: Option<u64>,
    scope: Option<BackendEventScope>,
    payload: String,
}

impl BackendEventFrame {
    #[must_use]
    pub fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    #[must_use]
    pub fn scope(&self) -> Option<BackendEventScope> {
        self.scope
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoredBackendEvent {
    sequence: u64,
    scope: BackendEventScope,
    payload: BackendEventPayload,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackendEventStreamService {
    #[serde(default = "new_stream_id")]
    stream_id: String,
    next_sequence: u64,
    events: Vec<StoredBackendEvent>,
    event_limit: usize,
}

impl Default for BackendEventStreamService {
    fn default() -> Self {
        Self::with_event_limit(DEFAULT_EVENT_LIMIT)
    }
}

impl BackendEventStreamService {
    pub fn with_event_limit(event_limit: usize) -> Self {
        Self {
            stream_id: new_stream_id(),
            next_sequence: 0,
            events: Vec::new(),
            event_limit,
        }
    }

    pub fn publish(&mut self, scope: BackendEventScope, payload: impl Into<String>) {
        self.publish_payload(scope, BackendEventPayload::from_text(payload));
    }

    pub fn publish_json(&mut self, scope: BackendEventScope, payload: Value) {
        self.publish_payload(scope, BackendEventPayload::from_json(payload));
    }

    fn publish_payload(&mut self, scope: BackendEventScope, payload: BackendEventPayload) {
        self.next_sequence += 1;
        self.events.push(StoredBackendEvent {
            sequence: self.next_sequence,
            scope,
            payload,
        });
        self.trim_old_events();
    }

    pub fn publish_terminal_started(
        &mut self,
        terminal_id: &str,
        workspace_id: &str,
        command: &str,
    ) {
        self.publish_json(
            BackendEventScope::Terminal,
            json!({
                "terminalId":terminal_id,"kind":"terminal.started",
                "eventId":format!("{terminal_id}.started"),
                "workspaceId":workspace_id,"command":command
            }),
        );
    }

    pub fn publish_terminal_output(
        &mut self,
        terminal_id: &str,
        stream: &str,
        output: &str,
        truncated: bool,
    ) {
        let redacted_output = redact_sensitive_with_status(output);
        let stdout = if stream == "stdout" {
            redacted_output.value()
        } else {
            ""
        };
        let stderr = if stream == "stderr" {
            redacted_output.value()
        } else {
            ""
        };
        self.publish_json(
            BackendEventScope::Terminal,
            json!({
                "terminalId":terminal_id,"kind":"terminal.output",
                "eventId":format!("{terminal_id}.output.{}", self.next_sequence + 1),
                "stdout":stdout,"stderr":stderr,"status":"exited","exitCode":null,
                "stdoutTruncated":truncated,"redacted":redacted_output.redacted()
            }),
        );
    }

    pub fn publish_terminal_completed(&mut self, terminal_id: &str, exit_code: Option<i32>) {
        self.publish_json(
            BackendEventScope::Terminal,
            json!({
                "terminalId":terminal_id,"kind":"terminal.completed",
                "eventId":format!("{terminal_id}.completed"),"stdout":"","stderr":"",
                "status":"exited","exitCode":exit_code,"stdoutTruncated":false,"redacted":false
            }),
        );
    }

    pub fn publish_agent_event(
        &mut self,
        kind: &str,
        workspace_id: &str,
        session_id: &str,
        backend_id: &str,
        message: &str,
    ) {
        self.publish_json(
            BackendEventScope::Session,
            json!({
                "kind":kind,"eventId":format!("{session_id}.{}", self.next_sequence + 1),
                "workspaceId":workspace_id,"sessionId":session_id,
                "backendId":backend_id,"message":message
            }),
        );
    }

    #[must_use]
    pub fn latest_sequence(&self) -> u64 {
        self.next_sequence
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("backend event outbox should serialize")
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let mut restored: Self =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        restored.event_limit = restored.event_limit.clamp(1, DEFAULT_EVENT_LIMIT);
        if restored.stream_id.is_empty() {
            restored.stream_id = new_stream_id();
        }
        restored.trim_old_events();
        let last_sequence = restored
            .events
            .last()
            .map(|event| event.sequence)
            .unwrap_or_default();
        restored.next_sequence = restored.next_sequence.max(last_sequence);
        Ok(restored)
    }

    #[must_use]
    pub fn heartbeat(&self) -> BackendEventFrame {
        BackendEventFrame {
            sequence: None,
            scope: None,
            payload: "heartbeat".to_string(),
        }
    }

    #[must_use]
    pub fn replay(&self, request: EventReplayRequest) -> EventReplayResponse {
        let reset_required = request
            .expected_stream_id_value()
            .is_some_and(|expected| expected != self.stream_id)
            || request.after_sequence_value() > self.next_sequence;
        let after_sequence = if reset_required {
            0
        } else {
            request.after_sequence_value()
        };
        let oldest_sequence = self.events.first().map(|event| event.sequence);
        let gap_detected =
            oldest_sequence.is_some_and(|oldest| oldest > after_sequence.saturating_add(1));
        let matching = self
            .events
            .iter()
            .filter(|event| event.sequence > after_sequence)
            .filter(|event| {
                request
                    .scope_value()
                    .is_none_or(|scope| scope == event.scope)
            })
            .collect::<Vec<_>>();
        let has_more = matching.len() > request.max_events_value();
        let frames = matching
            .into_iter()
            .take(request.max_events_value())
            .map(|event| BackendEventFrame {
                sequence: Some(event.sequence),
                scope: Some(event.scope),
                payload: event.payload.as_text(),
            })
            .collect::<Vec<_>>();
        let next_sequence = frames
            .last()
            .and_then(BackendEventFrame::sequence)
            .unwrap_or(after_sequence);
        EventReplayResponse::new(
            self.stream_id.clone(),
            oldest_sequence,
            self.next_sequence,
            next_sequence,
            has_more,
            gap_detected,
            reset_required,
            frames,
        )
    }

    fn trim_old_events(&mut self) {
        let overflow = self.events.len().saturating_sub(self.event_limit);
        if overflow > 0 {
            self.events.drain(0..overflow);
        }
    }
}

fn new_stream_id() -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "event-stream.{timestamp}.{}",
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}
