use std::time::{SystemTime, UNIX_EPOCH};

use desktoplab_redaction::redact_sensitive_bounded;
use desktoplab_tool_gateway::canonical_tool_mutates;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{IterativeStopReason, IterativeToolCall, ToolObservation};

const TRACE_SCHEMA_VERSION: u16 = 1;
const TRACE_DETAIL_LIMIT: usize = 512;
const TRACE_EVENT_LIMIT: usize = 2_048;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraceEvent {
    event_id: String,
    parent_event_id: Option<String>,
    correlation_id: Option<String>,
    sequence: usize,
    kind: String,
    recorded_at_unix_ms: u64,
    duration_ms: Option<u64>,
    source: String,
    mutation: bool,
    success: Option<bool>,
    truncated: bool,
    redacted: bool,
    detail: String,
}

impl AgentTraceEvent {
    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    #[must_use]
    pub fn parent_event_id(&self) -> Option<&str> {
        self.parent_event_id.as_deref()
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }

    #[must_use]
    pub fn success(&self) -> Option<bool> {
        self.success
    }

    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    #[must_use]
    pub fn mutation(&self) -> bool {
        self.mutation
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraceEnvelope {
    schema_version: u16,
    producer: String,
    session_id: String,
    started_at_unix_ms: u64,
    dropped_events: usize,
    events: Vec<AgentTraceEvent>,
}

impl Default for AgentTraceEnvelope {
    fn default() -> Self {
        Self::new("")
    }
}

impl AgentTraceEnvelope {
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            schema_version: TRACE_SCHEMA_VERSION,
            producer: format!("desktoplab-agent-engine/{}", env!("CARGO_PKG_VERSION")),
            session_id: session_id.into(),
            started_at_unix_ms: now_unix_ms(),
            dropped_events: 0,
            events: Vec::new(),
        }
    }

    #[must_use]
    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub fn events(&self) -> &[AgentTraceEvent] {
        &self.events
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut lines = Vec::with_capacity(self.events.len());
        for event in &self.events {
            lines.push(serde_json::to_string(&json!({
                "schemaVersion":self.schema_version,
                "producer":self.producer,
                "sessionId":self.session_id,
                "event":event
            }))?);
        }
        Ok(lines.join("\n"))
    }

    pub(crate) fn model_turn(&mut self, ordinal: usize) {
        self.push(
            "model_turn",
            None,
            "model",
            false,
            None,
            None,
            false,
            format!("ordinal={ordinal}"),
        );
    }

    pub(crate) fn tool_requested(&mut self, call: &IterativeToolCall) {
        let parent = self.last_kind_id("model_turn");
        self.push_with_parent(
            "tool_requested",
            parent,
            Some(call.id().to_string()),
            call.name(),
            tool_mutates(call.name()),
            None,
            None,
            false,
            format!("call_id={} tool={}", call.id(), call.name()),
        );
    }

    pub(crate) fn tool_observed(&mut self, observation: &ToolObservation, duration_ms: u64) {
        let parent = self.correlation_parent("tool_requested", observation.call_id());
        let provenance = observation.provenance();
        let detail = format!(
            "call_id={} evidence_id={} target={} exit_code={}",
            observation.call_id(),
            provenance.evidence_id(),
            safe_target(provenance.target()),
            provenance
                .exit_code()
                .map_or_else(|| "none".to_string(), |code| code.to_string())
        );
        self.push_with_parent(
            "tool_observed",
            parent,
            Some(observation.call_id().to_string()),
            observation.tool_name(),
            tool_mutates(observation.tool_name()),
            Some(observation.error().is_none()),
            Some(duration_ms),
            provenance.truncated(),
            detail,
        );
    }

    pub(crate) fn approval_required(&mut self, call_id: &str) {
        let parent = self.correlation_parent("tool_requested", call_id);
        self.push_with_parent(
            "approval_required",
            parent,
            Some(call_id.to_string()),
            "policy",
            false,
            None,
            None,
            false,
            format!("call_id={call_id}"),
        );
    }

    pub(crate) fn approval_resolved(&mut self, call_id: &str, decision: &str) {
        let parent = self.correlation_parent("approval_required", call_id);
        self.push_with_parent(
            "approval_resolved",
            parent,
            Some(call_id.to_string()),
            "policy",
            false,
            Some(decision == "approved"),
            None,
            false,
            format!("call_id={call_id} decision={decision}"),
        );
    }

    pub(crate) fn completed(&mut self) {
        self.push(
            "completed",
            None,
            "agent",
            false,
            Some(true),
            None,
            false,
            "final_response_recorded".to_string(),
        );
    }

    pub(crate) fn stopped(&mut self, reason: &IterativeStopReason) {
        self.push(
            "stopped",
            None,
            "agent",
            false,
            Some(false),
            None,
            false,
            format!("reason={}", reason.code()),
        );
    }

    fn push(
        &mut self,
        kind: &str,
        correlation_id: Option<String>,
        source: &str,
        mutation: bool,
        success: Option<bool>,
        duration_ms: Option<u64>,
        truncated: bool,
        detail: String,
    ) {
        let parent = self.events.last().map(|event| event.event_id.clone());
        self.push_with_parent(
            kind,
            parent,
            correlation_id,
            source,
            mutation,
            success,
            duration_ms,
            truncated,
            detail,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_with_parent(
        &mut self,
        kind: &str,
        parent_event_id: Option<String>,
        correlation_id: Option<String>,
        source: &str,
        mutation: bool,
        success: Option<bool>,
        duration_ms: Option<u64>,
        truncated: bool,
        detail: String,
    ) {
        if self.events.len() >= TRACE_EVENT_LIMIT {
            self.events.remove(0);
            self.dropped_events += 1;
        }
        let sequence = self.dropped_events + self.events.len() + 1;
        let redacted = redact_sensitive_bounded(&detail, TRACE_DETAIL_LIMIT);
        self.events.push(AgentTraceEvent {
            event_id: format!("{}:{sequence}", self.session_id),
            parent_event_id,
            correlation_id,
            sequence,
            kind: kind.to_string(),
            recorded_at_unix_ms: now_unix_ms(),
            duration_ms,
            source: source.to_string(),
            mutation,
            success,
            truncated,
            redacted: redacted.redacted(),
            detail: redacted.value().to_string(),
        });
    }

    fn last_kind_id(&self, kind: &str) -> Option<String> {
        self.events
            .iter()
            .rev()
            .find(|event| event.kind == kind)
            .map(|event| event.event_id.clone())
    }

    fn correlation_parent(&self, kind: &str, correlation_id: &str) -> Option<String> {
        self.events
            .iter()
            .rev()
            .find(|event| {
                event.kind == kind && event.correlation_id.as_deref() == Some(correlation_id)
            })
            .map(|event| event.event_id.clone())
    }
}

fn tool_mutates(name: &str) -> bool {
    canonical_tool_mutates(name)
}

fn safe_target(target: Option<&str>) -> &str {
    match target {
        Some(target) if std::path::Path::new(target).is_absolute() => "[ABSOLUTE_PATH_REDACTED]",
        Some(target) => target,
        None => "none",
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}
