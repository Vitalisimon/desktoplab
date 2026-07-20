use std::time::{Duration, Instant};

use crate::final_response::{self, FinalResponseError};
use crate::{
    IterativeLoopEvent, IterativeLoopState, IterativeLoopStatus, IterativeModelAdapter,
    IterativeModelDecision, IterativeStopReason, IterativeToolCall, IterativeToolExecutor,
    ToolObservation,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IterativeLoopLimits {
    max_turns: usize,
    max_tool_calls: usize,
    max_duration: Duration,
    max_identical_failures: usize,
}

impl IterativeLoopLimits {
    #[must_use]
    pub fn new(
        max_turns: usize,
        max_tool_calls: usize,
        max_duration: Duration,
        max_identical_failures: usize,
    ) -> Self {
        Self {
            max_turns,
            max_tool_calls,
            max_duration,
            max_identical_failures,
        }
    }
}

impl Default for IterativeLoopLimits {
    fn default() -> Self {
        Self::new(24, 24, Duration::from_secs(300), 3)
    }
}

pub struct IterativeAgentLoop {
    limits: IterativeLoopLimits,
}

impl IterativeAgentLoop {
    #[must_use]
    pub fn new(limits: IterativeLoopLimits) -> Self {
        Self { limits }
    }

    pub fn run(
        &self,
        state: &mut IterativeLoopState,
        model: &mut impl IterativeModelAdapter,
        executor: &mut impl IterativeToolExecutor,
    ) {
        let started_at = Instant::now();
        while state.status() == IterativeLoopStatus::Running {
            if started_at.elapsed() >= self.limits.max_duration {
                state.stop(
                    IterativeLoopStatus::Exhausted,
                    IterativeStopReason::MaxDuration,
                );
                break;
            }
            self.advance(state, model, executor);
        }
    }

    pub fn advance(
        &self,
        state: &mut IterativeLoopState,
        model: &mut impl IterativeModelAdapter,
        executor: &mut impl IterativeToolExecutor,
    ) {
        if !self.begin_model_turn(state) {
            return;
        }
        let decision = match model.decide(state) {
            Ok(decision) => decision,
            Err(reason) => {
                state.stop(
                    IterativeLoopStatus::Failed,
                    IterativeStopReason::ModelFailure(reason),
                );
                return;
            }
        };
        self.apply_model_decision(state, executor, decision);
    }

    pub fn begin_model_turn(&self, state: &mut IterativeLoopState) -> bool {
        if state.status() != IterativeLoopStatus::Running {
            return false;
        }
        if state.model_turns() >= self.limits.max_turns {
            state.stop(
                IterativeLoopStatus::Exhausted,
                IterativeStopReason::MaxTurns,
            );
            return false;
        }
        state.record_model_turn();
        true
    }

    pub fn apply_model_decision(
        &self,
        state: &mut IterativeLoopState,
        executor: &mut impl IterativeToolExecutor,
        decision: IterativeModelDecision,
    ) {
        if state.status() != IterativeLoopStatus::Running {
            return;
        }
        match decision {
            IterativeModelDecision::FinalResponse(response) if !response.trim().is_empty() => {
                match final_response::validate(&response, state) {
                    Ok(()) => state.complete(response),
                    Err(FinalResponseError::RawToolEnvelope) => state.stop(
                        IterativeLoopStatus::Failed,
                        IterativeStopReason::InvalidFinalResponse("raw_tool_envelope".to_string()),
                    ),
                    Err(FinalResponseError::UnsupportedTestClaim) => state.stop(
                        IterativeLoopStatus::Failed,
                        IterativeStopReason::UnsupportedTestClaim,
                    ),
                }
            }
            IterativeModelDecision::FinalResponse(_) => state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ModelFailure("empty_final_response".to_string()),
            ),
            IterativeModelDecision::Blocked(reason) => state.stop(
                IterativeLoopStatus::Blocked,
                IterativeStopReason::ModelBlocked(reason),
            ),
            IterativeModelDecision::Clarification {
                question,
                blocked_on,
            } => state.stop(
                IterativeLoopStatus::Blocked,
                IterativeStopReason::Clarification {
                    question,
                    blocked_on,
                },
            ),
            IterativeModelDecision::ToolCall(call) => self.execute_tool(state, executor, call),
        }
    }

    pub fn fail_model_turn(&self, state: &mut IterativeLoopState, reason: impl Into<String>) {
        if state.status() == IterativeLoopStatus::Running {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ModelFailure(reason.into()),
            );
        }
    }

    fn execute_tool(
        &self,
        state: &mut IterativeLoopState,
        executor: &mut impl IterativeToolExecutor,
        call: IterativeToolCall,
    ) {
        if state.has_observed(call.id()) {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::DuplicateToolCall(call.id().to_string()),
            );
            return;
        }
        if state.tool_calls() >= self.limits.max_tool_calls {
            state.stop(
                IterativeLoopStatus::Exhausted,
                IterativeStopReason::MaxToolCalls,
            );
            return;
        }
        state.record_tool_request(IterativeLoopEvent::ToolRequested { call: call.clone() });
        let tool_started_at = Instant::now();
        let observation = match executor.execute(&call) {
            Ok(observation) => observation,
            Err(reason) if reason == "approval_required" => {
                state.pause_for_approval(call);
                return;
            }
            Err(reason) if reason == "approval_denied" => {
                state.stop(
                    IterativeLoopStatus::Blocked,
                    IterativeStopReason::ApprovalDenied,
                );
                return;
            }
            Err(reason) => ToolObservation::failure(&call, reason),
        };
        state.record_observation(observation, elapsed_ms(tool_started_at));
        if self.repeated_failure_count(state, &call) >= self.limits.max_identical_failures {
            state.stop(
                IterativeLoopStatus::Exhausted,
                IterativeStopReason::RepeatedToolFailure,
            );
        }
    }

    fn repeated_failure_count(
        &self,
        state: &IterativeLoopState,
        call: &IterativeToolCall,
    ) -> usize {
        let Some(error) = state.observations().last().and_then(ToolObservation::error) else {
            return 0;
        };
        let signature = call.failure_signature(error);
        state
            .observations()
            .iter()
            .rev()
            .take_while(|observation| {
                observation.failure_signature().as_deref() == Some(&signature)
            })
            .count()
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

impl Default for IterativeAgentLoop {
    fn default() -> Self {
        Self::new(IterativeLoopLimits::default())
    }
}
