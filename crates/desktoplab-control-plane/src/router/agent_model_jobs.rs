use std::sync::{Arc, atomic::AtomicBool};

use super::agent_model_execution::{
    AgentModelExecutionError, PreparedAgentModelExecution, apply_model_execution_error,
};
use super::{ApiRouteResponse, LocalApiRouter};
use crate::agent_model_adapter::{
    backend_messages, decision_from_backend_output_with_registry, suppressed_tool_for_model_turn,
};
use desktoplab_agent_engine::IterativeLoopState;
use desktoplab_agent_engine::{IterativeAgentLoop, IterativeLoopStatus};
use desktoplab_agent_session::{SessionEvent, SessionState};

pub(crate) struct ClaimedAgentModelTurn {
    session_id: String,
    workspace_id: String,
    backend_id: String,
    execution: PreparedAgentModelExecution,
    cancellation: Arc<AtomicBool>,
    streaming: bool,
}

pub(crate) struct CompletedAgentModelTurn {
    session_id: String,
    workspace_id: String,
    backend_id: String,
    output: Result<String, AgentModelExecutionError>,
}

impl ClaimedAgentModelTurn {
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub(crate) fn backend_id(&self) -> &str {
        &self.backend_id
    }

    #[must_use]
    pub(crate) fn execute(self, mut on_delta: impl FnMut(&str)) -> CompletedAgentModelTurn {
        CompletedAgentModelTurn {
            session_id: self.session_id,
            workspace_id: self.workspace_id,
            backend_id: self.backend_id,
            output: self
                .execution
                .execute(&self.cancellation, self.streaming, &mut on_delta),
        }
    }
}

impl LocalApiRouter {
    pub(super) fn queue_native_iterative_session(
        &mut self,
        session_id: &str,
        workspace_id: &str,
        user_goal: &str,
        compiled_prompt: &str,
    ) -> ApiRouteResponse {
        self.sessions.append_events(
            session_id,
            &[
                SessionEvent::planning_started(user_goal),
                SessionEvent::execution_started(),
                SessionEvent::job_started(agent_job_id(session_id), current_timestamp(), true),
                SessionEvent::job_heartbeat(agent_job_id(session_id), current_timestamp()),
                SessionEvent::job_observation(
                    agent_job_id(session_id),
                    "Waiting for model execution",
                ),
            ],
        );
        self.agent_iterative_states
            .insert(session_id.to_string(), IterativeLoopState::new(session_id));
        self.agent_iterative_prompts
            .insert(session_id.to_string(), compiled_prompt.to_string());
        self.agent_iterative_event_offsets
            .insert(session_id.to_string(), 0);
        self.agent_cancellation_tokens
            .insert(session_id.to_string(), Arc::new(AtomicBool::new(false)));
        if let Err(error) = self.persist_agent_approval_journal() {
            return ApiRouteResponse::state_journal_failed(error);
        }
        self.native_iterative_session_response(session_id, workspace_id)
    }

    pub(crate) fn claim_next_agent_model_turn(&mut self) -> Option<ClaimedAgentModelTurn> {
        let session_id = self
            .agent_iterative_states
            .iter()
            .find_map(|(session_id, state)| {
                (state.status() == IterativeLoopStatus::Running
                    && self
                        .sessions
                        .get(session_id)
                        .is_some_and(|session| session.state() == SessionState::Running)
                    && !self.agent_model_inflight.contains(session_id))
                .then(|| session_id.clone())
            })?;
        let workspace_id = self.sessions.workspace_id_for(&session_id)?;
        let prompt = self.agent_iterative_prompts.get(&session_id)?.clone();
        let binding = self.agent_execution_bindings.get(&session_id).cloned();
        let registry = self
            .agent_tool_registry_for_model(binding.as_ref().and_then(|binding| binding.model_id()))
            .ok()?;
        let mut state = self.agent_iterative_states.remove(&session_id)?;
        if !IterativeAgentLoop::default().begin_model_turn(&mut state) {
            self.agent_iterative_states.insert(session_id, state);
            return None;
        }
        let suppressed_tool = suppressed_tool_for_model_turn(&state).map(str::to_string);
        let messages = backend_messages(&prompt, &state, &registry);
        let backend_id = self
            .sessions
            .get(&session_id)
            .map(|session| session.execution_backend_id().to_string())?;
        let execution = self.prepare_agent_model_execution(
            &backend_id,
            binding.as_ref(),
            messages,
            suppressed_tool.as_deref(),
        );
        let cancellation = self
            .agent_cancellation_tokens
            .entry(session_id.clone())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();
        let streaming = self.agent_streaming_sessions.contains(&session_id);
        self.sessions
            .heartbeat_job(&session_id, agent_job_id(&session_id), current_timestamp());
        self.sessions.observe_job(
            &session_id,
            agent_job_id(&session_id),
            "Model execution started",
        );
        self.agent_iterative_states
            .insert(session_id.clone(), state);
        self.agent_model_inflight.insert(session_id.clone());
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
            self.agent_model_inflight.remove(&session_id);
            return None;
        }
        Some(ClaimedAgentModelTurn {
            session_id,
            workspace_id,
            backend_id,
            execution,
            cancellation,
            streaming,
        })
    }

    pub(crate) fn record_agent_model_delta(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        backend_id: &str,
        delta: &str,
    ) {
        if delta.is_empty()
            || !self.agent_streaming_sessions.contains(session_id)
            || self.sessions.get(session_id).is_some_and(|session| {
                session.state() == desktoplab_agent_session::SessionState::Cancelled
            })
        {
            return;
        }
        self.events.publish_agent_event(
            "agent.stream.delta",
            workspace_id,
            session_id,
            backend_id,
            delta,
        );
        self.persist_event_outbox();
    }

    pub(crate) fn complete_agent_model_turn(&mut self, completed: CompletedAgentModelTurn) {
        self.agent_model_inflight.remove(&completed.session_id);
        let session_state = self
            .sessions
            .get(&completed.session_id)
            .map(|session| session.state());
        if session_state == Some(SessionState::Paused) {
            return;
        }
        if !matches!(session_state, Some(SessionState::Running)) {
            self.agent_iterative_states.remove(&completed.session_id);
            self.agent_iterative_prompts.remove(&completed.session_id);
            self.agent_iterative_event_offsets
                .remove(&completed.session_id);
            self.agent_cancellation_tokens.remove(&completed.session_id);
            return;
        }
        let Some(mut state) = self.agent_iterative_states.remove(&completed.session_id) else {
            return;
        };
        let model_id = self
            .agent_execution_bindings
            .get(&completed.session_id)
            .and_then(|binding| binding.model_id());
        let registry = match self.agent_tool_registry_for_model(model_id) {
            Ok(registry) => registry,
            Err(error) => {
                IterativeAgentLoop::default().fail_model_turn(&mut state, error);
                let _ = self.apply_native_iterative_state(
                    state,
                    &completed.workspace_id,
                    &completed.backend_id,
                    "",
                );
                return;
            }
        };
        let agent_loop = IterativeAgentLoop::default();
        match completed.output {
            Ok(output) => {
                match decision_from_backend_output_with_registry(&state, &output, registry) {
                    Ok(decision) => {
                        state.clear_model_protocol_recovery();
                        if let Err(error) = self.apply_router_agent_decision(
                            &mut state,
                            &completed.workspace_id,
                            decision,
                        ) {
                            agent_loop.fail_model_turn(&mut state, error);
                        }
                    }
                    Err(error) => {
                        if !state.request_model_protocol_retry(error.clone()) {
                            agent_loop.fail_model_turn(
                                &mut state,
                                format!("model_protocol_error:{error}"),
                            );
                        }
                    }
                }
            }
            Err(error) => apply_model_execution_error(&mut state, &agent_loop, error),
        }
        let _ = self.apply_native_iterative_state(
            state,
            &completed.workspace_id,
            &completed.backend_id,
            "",
        );
        self.persist_event_outbox();
    }
}

fn agent_job_id(session_id: &str) -> String {
    format!("agent-job.{session_id}")
}

fn current_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::IterativeLoopState;

    use super::LocalApiRouter;
    use crate::router::WorkspaceRecord;

    #[test]
    fn model_turn_keeps_the_workspace_owned_by_its_session() {
        let mut router = LocalApiRouter::default();
        let session = router
            .sessions
            .create_session("workspace.a", "backend.ollama");
        let session_id = session.session_id().to_string();
        router.sessions.start(&session_id);
        router
            .agent_iterative_states
            .insert(session_id.clone(), IterativeLoopState::new(&session_id));
        router
            .agent_iterative_prompts
            .insert(session_id, "prompt".to_string());
        router.workspace = Some(WorkspaceRecord {
            workspace_id: "workspace.b".to_string(),
            display_name: "B".to_string(),
            root_path: "/tmp/workspace-b".to_string(),
        });

        let claimed = router.claim_next_agent_model_turn().unwrap();

        assert_eq!(claimed.workspace_id(), "workspace.a");
    }
}
