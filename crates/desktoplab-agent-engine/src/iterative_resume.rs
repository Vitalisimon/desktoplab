use std::time::Instant;

use crate::{
    IterativeAgentLoop, IterativeApproval, IterativeApprovalDecision, IterativeLoopState,
    IterativeLoopStatus, IterativeModelAdapter, IterativeStopReason, IterativeToolExecutor,
    ToolObservation,
};

impl IterativeAgentLoop {
    pub fn resume_with_approval(
        &self,
        state: &mut IterativeLoopState,
        approval: IterativeApproval,
        model: &mut impl IterativeModelAdapter,
        executor: &mut impl IterativeToolExecutor,
    ) {
        if state.status() != IterativeLoopStatus::WaitingForApproval {
            return;
        }
        let Some(pending) = state.pending_approval().cloned() else {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ApprovalPayloadMismatch,
            );
            return;
        };
        if !approval.matches(&pending) {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ApprovalPayloadMismatch,
            );
            return;
        }
        let Some(pending) = state.take_pending_approval() else {
            return;
        };
        match approval.decision() {
            IterativeApprovalDecision::Denied => {
                state.record_approval_resolution(pending.call_id(), approval.decision());
                state.stop(
                    IterativeLoopStatus::Blocked,
                    IterativeStopReason::ApprovalDenied,
                );
            }
            IterativeApprovalDecision::Expired => {
                state.record_approval_resolution(pending.call_id(), approval.decision());
                state.stop(
                    IterativeLoopStatus::Blocked,
                    IterativeStopReason::ApprovalExpired,
                );
            }
            IterativeApprovalDecision::Approved => {
                state.resume_after_approval(pending.call_id(), approval.decision());
                let tool_started_at = Instant::now();
                match executor.execute_approved(pending.call()) {
                    Ok(observation) => {
                        state.record_observation(observation, elapsed_ms(tool_started_at))
                    }
                    Err(reason) if reason == "approval_required" => state.stop(
                        IterativeLoopStatus::Failed,
                        IterativeStopReason::ModelFailure("approval_was_not_consumed".to_string()),
                    ),
                    Err(reason) => state.record_observation(
                        ToolObservation::failure(pending.call(), reason),
                        elapsed_ms(tool_started_at),
                    ),
                }
                self.run(state, model, executor);
            }
        }
    }

    pub fn resume_with_approved_observation(
        &self,
        state: &mut IterativeLoopState,
        approval: IterativeApproval,
        observation: ToolObservation,
        model: &mut impl IterativeModelAdapter,
        executor: &mut impl IterativeToolExecutor,
    ) {
        if approval.decision() != IterativeApprovalDecision::Approved {
            self.resume_with_approval(state, approval, model, executor);
            return;
        }
        if !self.accept_approved_observation(state, approval, observation) {
            return;
        }
        self.run(state, model, executor);
    }

    pub fn accept_approved_observation(
        &self,
        state: &mut IterativeLoopState,
        approval: IterativeApproval,
        observation: ToolObservation,
    ) -> bool {
        if state.status() != IterativeLoopStatus::WaitingForApproval {
            return false;
        }
        let Some(pending) = state.pending_approval().cloned() else {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ApprovalPayloadMismatch,
            );
            return false;
        };
        if approval.decision() != IterativeApprovalDecision::Approved
            || !approval.matches(&pending)
            || observation.call_id() != pending.call_id()
            || observation.tool_name() != pending.call().name()
        {
            state.stop(
                IterativeLoopStatus::Failed,
                IterativeStopReason::ApprovalPayloadMismatch,
            );
            return false;
        }
        let Some(pending) = state.take_pending_approval() else {
            return false;
        };
        state.resume_after_approval(pending.call_id(), approval.decision());
        state.record_observation(observation, 0);
        true
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
