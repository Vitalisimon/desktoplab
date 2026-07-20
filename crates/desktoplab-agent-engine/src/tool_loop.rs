use std::time::{Duration, Instant};

use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent, ToolOutcome};

use crate::loop_events::tool_evidence;
use crate::tool_decisions;
use crate::tool_failure_guard::{self, RepeatedFailureDetector};
use crate::tool_telemetry;
use crate::{AgentEvidence, ApprovalDecision, PlannedToolCall};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolLoopStopReason {
    ApprovalRequired,
    ApprovalDenied,
    MaxSteps,
    MaxToolCalls,
    MaxDuration,
    PolicyBlocked,
    RepeatedToolFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolLoopLimits {
    max_steps: usize,
    max_tool_calls: usize,
    max_duration: Duration,
}

impl ToolLoopLimits {
    #[must_use]
    pub fn new(max_steps: usize, max_tool_calls: usize, max_duration: Duration) -> Self {
        Self {
            max_steps,
            max_tool_calls,
            max_duration,
        }
    }
}

impl Default for ToolLoopLimits {
    fn default() -> Self {
        Self::new(12, 12, Duration::from_secs(30))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolLoopStep {
    intent: ToolIntent,
    observation: Option<String>,
}

impl ToolLoopStep {
    #[must_use]
    pub fn new(intent: ToolIntent) -> Self {
        Self {
            intent,
            observation: None,
        }
    }

    #[must_use]
    pub fn with_observation(mut self, observation: impl Into<String>) -> Self {
        self.observation = Some(observation.into());
        self
    }

    pub(crate) fn from_planned(tool_call: &PlannedToolCall) -> Self {
        Self::new(tool_call.intent().clone()).with_observation(format!(
            "tool {} completed",
            tool_evidence(tool_call.intent())
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolLoopRunResult {
    pending_approvals: usize,
    evidence: Vec<AgentEvidence>,
    stop_reason: Option<ToolLoopStopReason>,
}

impl ToolLoopRunResult {
    #[must_use]
    pub fn pending_approvals(&self) -> usize {
        self.pending_approvals
    }

    #[must_use]
    pub fn evidence(&self) -> &[AgentEvidence] {
        &self.evidence
    }

    #[must_use]
    pub fn stop_reason(&self) -> Option<ToolLoopStopReason> {
        self.stop_reason
    }
}

pub struct BoundedToolLoop {
    gateway: ToolGateway,
    limits: ToolLoopLimits,
}

impl BoundedToolLoop {
    #[must_use]
    pub fn new(gateway: ToolGateway, limits: ToolLoopLimits) -> Self {
        Self { gateway, limits }
    }

    pub fn run(
        &mut self,
        session: &mut AgentSession,
        events: &mut Vec<SessionEvent>,
        steps: &[ToolLoopStep],
        approval: ApprovalDecision,
    ) -> ToolLoopRunResult {
        let started_at = Instant::now();
        let mut evidence = Vec::new();
        let approval_mode = self.gateway.approval_mode().as_str();
        let mut repeated_failure = RepeatedFailureDetector::default();

        for (step_index, step) in steps.iter().enumerate() {
            if let Some(reason) = self.limit_stop_reason(step_index, step_index, started_at) {
                return self.stop(session, events, 0, evidence, reason);
            }
            tool_decisions::record(session, events, &step.intent, "planned", approval_mode);
            let outcome = self.gateway.authorize(step.intent.clone());
            let ordinal = step_index + 1;
            tool_telemetry::record_before_tool(
                session,
                events,
                &step.intent,
                &outcome,
                approval,
                ordinal,
                approval_mode,
            );
            match outcome {
                ToolOutcome::Allowed(_) => {
                    self.record_executed(session, events, step, approval_mode, &mut evidence);
                }
                ToolOutcome::ApprovalRequired(_) if approval == ApprovalDecision::Approved => {
                    tool_decisions::record(
                        session,
                        events,
                        &step.intent,
                        "approved",
                        approval_mode,
                    );
                    self.record_executed(session, events, step, approval_mode, &mut evidence);
                }
                ToolOutcome::ApprovalRequired(_) if approval == ApprovalDecision::Denied => {
                    evidence.push(AgentEvidence::ApprovalDenied);
                    tool_decisions::record(session, events, &step.intent, "blocked", approval_mode);
                    return self.stop(
                        session,
                        events,
                        0,
                        evidence,
                        ToolLoopStopReason::ApprovalDenied,
                    );
                }
                ToolOutcome::ApprovalRequired(_) => {
                    tool_decisions::record(
                        session,
                        events,
                        &step.intent,
                        "approval_required",
                        approval_mode,
                    );
                    return self.stop(
                        session,
                        events,
                        1,
                        evidence,
                        ToolLoopStopReason::ApprovalRequired,
                    );
                }
                ToolOutcome::Blocked(ref reason) => {
                    tool_decisions::record_with_reason(
                        session,
                        events,
                        &step.intent,
                        "blocked",
                        approval_mode,
                        &reason,
                    );
                    apply_event(session, events, SessionEvent::blocked(reason));
                    return ToolLoopRunResult {
                        pending_approvals: 0,
                        evidence,
                        stop_reason: Some(ToolLoopStopReason::PolicyBlocked),
                    };
                }
            }
            if repeated_failure
                .observed(&step.intent, step.observation.as_deref())
                .is_some_and(|count| count >= 3)
            {
                return self.stop(
                    session,
                    events,
                    0,
                    evidence,
                    ToolLoopStopReason::RepeatedToolFailure,
                );
            }
        }

        ToolLoopRunResult {
            pending_approvals: 0,
            evidence,
            stop_reason: None,
        }
    }

    fn record_executed(
        &self,
        session: &mut AgentSession,
        events: &mut Vec<SessionEvent>,
        step: &ToolLoopStep,
        approval_mode: &str,
        evidence: &mut Vec<AgentEvidence>,
    ) {
        tool_decisions::record(session, events, &step.intent, "executed", approval_mode);
        evidence.push(AgentEvidence::ToolExecuted(tool_evidence(&step.intent)));
        if let Some(observation) = &step.observation {
            apply_event(
                session,
                events,
                SessionEvent::backend_response_received(format!(
                    "Observation: {}",
                    tool_telemetry::bounded_observation(observation)
                )),
            );
        }
    }

    fn limit_stop_reason(
        &self,
        step_count: usize,
        tool_count: usize,
        started_at: Instant,
    ) -> Option<ToolLoopStopReason> {
        if step_count >= self.limits.max_steps {
            Some(ToolLoopStopReason::MaxSteps)
        } else if tool_count >= self.limits.max_tool_calls {
            Some(ToolLoopStopReason::MaxToolCalls)
        } else if started_at.elapsed() >= self.limits.max_duration {
            Some(ToolLoopStopReason::MaxDuration)
        } else {
            None
        }
    }

    fn stop(
        &self,
        session: &mut AgentSession,
        events: &mut Vec<SessionEvent>,
        pending_approvals: usize,
        evidence: Vec<AgentEvidence>,
        reason: ToolLoopStopReason,
    ) -> ToolLoopRunResult {
        apply_event(
            session,
            events,
            SessionEvent::blocked(tool_failure_guard::blocked_reason(reason)),
        );
        ToolLoopRunResult {
            pending_approvals,
            evidence,
            stop_reason: Some(reason),
        }
    }
}

fn apply_event(session: &mut AgentSession, events: &mut Vec<SessionEvent>, event: SessionEvent) {
    session.apply(event.clone());
    events.push(event);
}
