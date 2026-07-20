use desktoplab_agent_engine::{IterativeAgentLoop, IterativeApproval, IterativeLoopState};
use desktoplab_agent_session::SessionEvent;

use super::agent_iterative::iterative_tool_decision;
use super::agent_pending::PendingAgentAction;
use super::agent_sessions::PendingExecutionOutcome;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(super) fn continue_native_iterative_after_execution(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
        pending: &PendingAgentAction,
        outcome: PendingExecutionOutcome,
    ) -> ApiRouteResponse {
        let Some(mut state) = self.accept_native_execution(pending, outcome) else {
            return self.native_iterative_session_response(pending.session_id(), workspace_id);
        };
        let session_id = state.session_id().to_string();
        self.agent_iterative_states.remove(&session_id);
        let prompt = self
            .agent_iterative_prompts
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        if let Err(error) =
            self.run_native_iterative_until_pause(&mut state, &prompt, workspace_id, backend_id)
        {
            self.sessions.fail(&session_id, error);
            return self.native_iterative_session_response(&session_id, workspace_id);
        }
        self.apply_native_iterative_state(state, workspace_id, backend_id, "")
    }

    pub(super) fn defer_native_iterative_after_execution(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
        pending: &PendingAgentAction,
        outcome: PendingExecutionOutcome,
    ) -> ApiRouteResponse {
        let Some(state) = self.accept_native_execution(pending, outcome) else {
            return self.native_iterative_session_response(pending.session_id(), workspace_id);
        };
        self.apply_native_iterative_state(state, workspace_id, backend_id, "")
    }

    fn accept_native_execution(
        &mut self,
        pending: &PendingAgentAction,
        outcome: PendingExecutionOutcome,
    ) -> Option<IterativeLoopState> {
        let session_id = pending.session_id().to_string();
        let mut state = self.agent_iterative_states.remove(&session_id)?;
        let iterative_pending = state.pending_approval().cloned()?;
        let call = iterative_pending.call().clone();
        let observation = match outcome {
            PendingExecutionOutcome::NativeObservation(observation) => observation,
            PendingExecutionOutcome::Failed(reason) => {
                self.sessions.fail(&session_id, reason);
                return None;
            }
            #[cfg(debug_assertions)]
            _ => {
                self.sessions
                    .fail(&session_id, "native_executor_observation_missing");
                return None;
            }
        };
        let applied = observation.error().is_none();
        let approval = IterativeApproval::approved(
            iterative_pending.call_id(),
            iterative_pending.payload_fingerprint(),
        );
        if !IterativeAgentLoop::default().accept_approved_observation(
            &mut state,
            approval,
            observation,
        ) {
            self.sessions
                .fail(&session_id, "iterative_approval_payload_mismatch");
            return None;
        }
        if let Some(action) = self.agent_pending_actions.get_mut(pending.approval_id()) {
            if applied {
                action.mark_applied();
            } else {
                action.mark_failed();
            }
        }
        self.sessions.append_events(
            &session_id,
            &[
                SessionEvent::resumed(),
                SessionEvent::tool_decision_recorded(iterative_tool_decision(
                    "approved",
                    call.name(),
                    call.id(),
                )),
                SessionEvent::tool_decision_recorded(iterative_tool_decision(
                    if applied { "executed" } else { "failed" },
                    call.name(),
                    call.id(),
                )),
            ],
        );
        self.agent_iterative_states
            .insert(session_id.clone(), state.clone());
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
            return None;
        }
        Some(state)
    }
}
