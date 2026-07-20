use serde::{Deserialize, Serialize};

use crate::{
    AgentTraceEnvelope, IterativeApprovalDecision, IterativeToolCall, PendingToolApproval,
};
use crate::{IterativeLoopEvent, IterativeLoopStatus, IterativeStopReason, ToolObservation};

const MAX_DISTINCT_MODEL_PROTOCOL_RETRIES: usize = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IterativeLoopState {
    session_id: String,
    status: IterativeLoopStatus,
    model_turns: usize,
    tool_calls: usize,
    observations: Vec<ToolObservation>,
    events: Vec<IterativeLoopEvent>,
    final_response: Option<String>,
    stop_reason: Option<IterativeStopReason>,
    #[serde(default)]
    pending_approval: Option<PendingToolApproval>,
    #[serde(default)]
    model_protocol_retry_count: usize,
    #[serde(default)]
    model_protocol_recovery: Option<String>,
    #[serde(default)]
    trace: AgentTraceEnvelope,
}

impl IterativeLoopState {
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        Self {
            trace: AgentTraceEnvelope::new(session_id.clone()),
            session_id,
            status: IterativeLoopStatus::Running,
            model_turns: 0,
            tool_calls: 0,
            observations: Vec::new(),
            events: Vec::new(),
            final_response: None,
            stop_reason: None,
            pending_approval: None,
            model_protocol_retry_count: 0,
            model_protocol_recovery: None,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(value: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(value)
    }

    pub fn cancel(&mut self, reason: impl Into<String>) {
        self.stop(
            IterativeLoopStatus::Cancelled,
            IterativeStopReason::Cancelled(reason.into()),
        );
    }

    #[must_use]
    pub fn status(&self) -> IterativeLoopStatus {
        self.status
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn model_turns(&self) -> usize {
        self.model_turns
    }

    #[must_use]
    pub fn tool_calls(&self) -> usize {
        self.tool_calls
    }

    #[must_use]
    pub fn observations(&self) -> &[ToolObservation] {
        &self.observations
    }

    #[must_use]
    pub fn events(&self) -> &[IterativeLoopEvent] {
        &self.events
    }

    #[must_use]
    pub fn final_response(&self) -> Option<&str> {
        self.final_response.as_deref()
    }

    #[must_use]
    pub fn stop_reason_code(&self) -> Option<&'static str> {
        self.stop_reason.as_ref().map(IterativeStopReason::code)
    }

    #[must_use]
    pub fn stop_reason(&self) -> Option<&IterativeStopReason> {
        self.stop_reason.as_ref()
    }

    #[must_use]
    pub fn user_block_reason(&self) -> Option<String> {
        match self.stop_reason.as_ref()? {
            IterativeStopReason::Clarification { question, .. } => {
                Some(format!("clarification_required:{question}"))
            }
            IterativeStopReason::ModelBlocked(reason) => Some(reason.clone()),
            reason => Some(reason.code().to_string()),
        }
    }

    #[must_use]
    pub fn user_failure_reason(&self) -> Option<String> {
        match self.stop_reason.as_ref()? {
            IterativeStopReason::ModelFailure(reason) => Some(format!("model_failure:{reason}")),
            IterativeStopReason::InvalidFinalResponse(reason) => {
                Some(format!("invalid_final_response:{reason}"))
            }
            IterativeStopReason::DuplicateToolCall(call_id) => {
                Some(format!("duplicate_tool_call:{call_id}"))
            }
            reason => Some(reason.code().to_string()),
        }
    }

    #[must_use]
    pub fn model_protocol_recovery(&self) -> Option<&str> {
        self.model_protocol_recovery.as_deref()
    }

    pub fn request_model_protocol_retry(&mut self, reason: impl Into<String>) -> bool {
        if self.status != IterativeLoopStatus::Running {
            return false;
        }
        let reason = reason.into();
        if self.model_protocol_retry_count >= MAX_DISTINCT_MODEL_PROTOCOL_RETRIES
            || self.model_protocol_recovery.as_deref() == Some(reason.as_str())
        {
            return false;
        }
        self.model_protocol_retry_count += 1;
        self.model_protocol_recovery = Some(reason.clone());
        self.events.push(IterativeLoopEvent::ModelProtocolRetry {
            ordinal: self.model_protocol_retry_count,
            reason,
        });
        true
    }

    pub fn clear_model_protocol_recovery(&mut self) {
        self.model_protocol_retry_count = 0;
        self.model_protocol_recovery = None;
    }

    #[must_use]
    pub fn clarification(&self) -> Option<(&str, &str)> {
        match self.stop_reason.as_ref()? {
            IterativeStopReason::Clarification {
                question,
                blocked_on,
            } => Some((question, blocked_on)),
            _ => None,
        }
    }

    #[must_use]
    pub fn pending_approval(&self) -> Option<&PendingToolApproval> {
        self.pending_approval.as_ref()
    }

    #[must_use]
    pub fn trace(&self) -> &AgentTraceEnvelope {
        &self.trace
    }

    pub(crate) fn has_passing_test_evidence(&self) -> bool {
        self.observations
            .iter()
            .any(ToolObservation::is_passing_test_evidence)
    }

    pub(crate) fn has_observed(&self, call_id: &str) -> bool {
        self.observations
            .iter()
            .any(|observation| observation.call_id() == call_id)
    }

    pub(crate) fn record_model_turn(&mut self) {
        self.model_turns += 1;
        self.trace.model_turn(self.model_turns);
        self.events.push(IterativeLoopEvent::ModelTurn {
            ordinal: self.model_turns,
        });
    }

    pub(crate) fn record_tool_request(&mut self, event: IterativeLoopEvent) {
        self.tool_calls += 1;
        if let IterativeLoopEvent::ToolRequested { call } = &event {
            self.trace.tool_requested(call);
        }
        self.events.push(event);
    }

    pub(crate) fn record_observation(&mut self, observation: ToolObservation, duration_ms: u64) {
        self.trace.tool_observed(&observation, duration_ms);
        self.events.push(IterativeLoopEvent::ToolObserved {
            observation: observation.clone(),
        });
        self.observations.push(observation);
    }

    pub(crate) fn pause_for_approval(&mut self, call: IterativeToolCall) {
        let pending = PendingToolApproval::new(call);
        self.status = IterativeLoopStatus::WaitingForApproval;
        self.stop_reason = Some(IterativeStopReason::ApprovalRequired);
        self.events.push(IterativeLoopEvent::ApprovalRequired {
            call_id: pending.call_id().to_string(),
            payload_fingerprint: pending.payload_fingerprint().to_string(),
        });
        self.trace.approval_required(pending.call_id());
        self.pending_approval = Some(pending);
    }

    pub(crate) fn take_pending_approval(&mut self) -> Option<PendingToolApproval> {
        self.pending_approval.take()
    }

    pub(crate) fn resume_after_approval(
        &mut self,
        call_id: &str,
        decision: IterativeApprovalDecision,
    ) {
        self.status = IterativeLoopStatus::Running;
        self.stop_reason = None;
        self.record_approval_resolution(call_id, decision);
    }

    pub(crate) fn record_approval_resolution(
        &mut self,
        call_id: &str,
        decision: IterativeApprovalDecision,
    ) {
        self.trace.approval_resolved(call_id, decision.label());
        self.events.push(IterativeLoopEvent::ApprovalResolved {
            call_id: call_id.to_string(),
            decision: decision.label().to_string(),
        });
    }

    pub(crate) fn complete(&mut self, response: String) {
        self.final_response = Some(response.clone());
        self.status = IterativeLoopStatus::Completed;
        self.stop_reason = Some(IterativeStopReason::FinalResponse);
        self.trace.completed();
        self.events.push(IterativeLoopEvent::Completed { response });
    }

    pub(crate) fn stop(&mut self, status: IterativeLoopStatus, reason: IterativeStopReason) {
        self.status = status;
        self.stop_reason = Some(reason.clone());
        self.trace.stopped(&reason);
        self.events.push(IterativeLoopEvent::Stopped { reason });
    }
}
