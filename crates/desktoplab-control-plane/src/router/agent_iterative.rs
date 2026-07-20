use std::path::Path;

use desktoplab_agent_engine::{
    IterativeLoopEvent, IterativeLoopState, IterativeLoopStatus, ToolObservation,
};
use desktoplab_agent_session::{SessionEvent, TerminalEvidence};
use desktoplab_tool_gateway::canonical_tool_mutates;

use super::agent_observation_display::readable_observation;
use super::agent_pending::canonical_tool_intent;
use super::agent_sessions::AgentApprovalRequestOutcome;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(super) fn backend_uses_native_iterative_loop(&self, backend_id: &str) -> bool {
        matches!(
            backend_id,
            "backend.ollama"
                | "backend.mlx-lm"
                | "backend.lm-studio"
                | "backend.high-end-local"
                | "backend.codex"
        ) && match self.agent_backend_execution {
            super::AgentBackendExecutionMode::Execute => true,
            #[cfg(debug_assertions)]
            super::AgentBackendExecutionMode::NativeIterativeSequenceForTest(_) => true,
            #[cfg(debug_assertions)]
            _ => false,
        }
    }

    pub(super) fn start_native_iterative_session(
        &mut self,
        session_id: &str,
        workspace_id: &str,
        backend_id: &str,
        user_goal: &str,
        compiled_prompt: &str,
    ) -> ApiRouteResponse {
        let mut state = IterativeLoopState::new(session_id);
        self.agent_iterative_prompts
            .insert(session_id.to_string(), compiled_prompt.to_string());
        self.sessions.append_events(
            session_id,
            &[
                SessionEvent::planning_started(user_goal),
                SessionEvent::execution_started(),
            ],
        );
        if let Err(error) = self.run_native_iterative_until_pause(
            &mut state,
            compiled_prompt,
            workspace_id,
            backend_id,
        ) {
            self.sessions.fail(session_id, error);
            return self.native_iterative_session_response(session_id, workspace_id);
        }

        self.apply_native_iterative_state(state, workspace_id, backend_id, user_goal)
    }

    pub(super) fn apply_native_iterative_state(
        &mut self,
        state: IterativeLoopState,
        workspace_id: &str,
        backend_id: &str,
        user_goal: &str,
    ) -> ApiRouteResponse {
        let session_id = state.session_id().to_string();
        self.append_native_iterative_events(&state);
        match state.status() {
            IterativeLoopStatus::WaitingForApproval => {
                let Some(pending) = state.pending_approval().cloned() else {
                    self.sessions
                        .fail(&session_id, "iterative_approval_state_missing");
                    return self.native_iterative_session_response(&session_id, workspace_id);
                };
                let call = pending.call();
                let Some(arguments) = call.arguments().as_object() else {
                    self.sessions
                        .fail(&session_id, "canonical_tool_arguments_invalid");
                    return self.native_iterative_session_response(&session_id, workspace_id);
                };
                let Some(intent) = canonical_tool_intent(call.name(), arguments) else {
                    self.sessions
                        .fail(&session_id, "unsupported_canonical_tool");
                    return self.native_iterative_session_response(&session_id, workspace_id);
                };
                let Some(session) = self.sessions.get(&session_id) else {
                    return ApiRouteResponse::not_found();
                };
                let Some(workspace) = self.execution_workspace_record(&session_id) else {
                    self.sessions
                        .fail(&session_id, "execution_workspace_unavailable");
                    return self.native_iterative_session_response(&session_id, workspace_id);
                };
                self.agent_iterative_states
                    .insert(session_id.clone(), state);
                self.remember_iterative_event_offset(&session_id);
                let approval = self.request_agent_tool_approval(
                    &session_id,
                    Some(&intent),
                    &session,
                    None,
                    user_goal,
                    Path::new(&workspace.root_path),
                );
                match approval {
                    Ok(
                        AgentApprovalRequestOutcome::Created
                        | AgentApprovalRequestOutcome::Deduplicated,
                    ) => self.sessions.block(&session_id, "waiting for approval"),
                    Ok(AgentApprovalRequestOutcome::CheckpointBlocked) => self
                        .sessions
                        .block(&session_id, "checkpoint blocked risky mutation"),
                    Ok(AgentApprovalRequestOutcome::Skipped) => {
                        self.sessions.fail(&session_id, "approval_boundary_missing")
                    }
                    #[cfg(debug_assertions)]
                    Ok(AgentApprovalRequestOutcome::Malformed) => self
                        .sessions
                        .fail(&session_id, "malformed structured tool action"),
                    #[cfg(debug_assertions)]
                    Ok(AgentApprovalRequestOutcome::PersistenceFailed) => {
                        self.sessions.fail(&session_id, "state_journal_failed")
                    }
                    Err(error) => return ApiRouteResponse::state_journal_failed(error),
                }
            }
            IterativeLoopStatus::Completed => {
                let response = state.final_response().unwrap_or_default().to_string();
                self.sessions.append_events(
                    &session_id,
                    &[
                        SessionEvent::backend_response_received(response.clone()),
                        SessionEvent::completed(response),
                    ],
                );
                self.agent_iterative_states.remove(&session_id);
                self.agent_iterative_event_offsets.remove(&session_id);
                self.agent_iterative_prompts.remove(&session_id);
                if self.agent_streaming_sessions.remove(&session_id) {
                    self.events.publish_agent_event(
                        "agent.stream.completed",
                        workspace_id,
                        &session_id,
                        backend_id,
                        "Streaming response completed",
                    );
                }
            }
            IterativeLoopStatus::Blocked => {
                let reason = state.user_block_reason().unwrap_or_else(|| {
                    state
                        .stop_reason_code()
                        .unwrap_or("agent_blocked")
                        .to_string()
                });
                self.sessions.block(&session_id, reason);
                self.agent_iterative_states
                    .insert(session_id.clone(), state);
                self.remember_iterative_event_offset(&session_id);
            }
            IterativeLoopStatus::Failed | IterativeLoopStatus::Exhausted => {
                let reason = state
                    .user_failure_reason()
                    .unwrap_or_else(|| "agent_failed".to_string());
                self.sessions.fail(&session_id, reason);
                self.agent_iterative_states
                    .insert(session_id.clone(), state);
                self.remember_iterative_event_offset(&session_id);
            }
            IterativeLoopStatus::Cancelled => {
                self.sessions.cancel(
                    &session_id,
                    state.stop_reason_code().unwrap_or("agent_cancelled"),
                );
                self.agent_iterative_states
                    .insert(session_id.clone(), state);
                self.remember_iterative_event_offset(&session_id);
            }
            IterativeLoopStatus::Running => {
                self.agent_iterative_states
                    .insert(session_id.clone(), state);
                self.remember_iterative_event_offset(&session_id);
            }
        }
        if let Err(error) = self.persist_agent_approval_journal() {
            return ApiRouteResponse::state_journal_failed(error);
        }
        self.events.publish_agent_event(
            native_event_kind(
                self.sessions
                    .get(&session_id)
                    .map(|session| session.state()),
            ),
            workspace_id,
            &session_id,
            backend_id,
            "Agent step updated",
        );
        self.native_iterative_session_response(&session_id, workspace_id)
    }

    fn append_native_iterative_events(&mut self, state: &IterativeLoopState) {
        let offset = self
            .agent_iterative_event_offsets
            .get(state.session_id())
            .copied()
            .unwrap_or(0);
        let mut events = Vec::new();
        for event in state.events().iter().skip(offset) {
            match event {
                IterativeLoopEvent::ToolRequested { call } => {
                    events.push(SessionEvent::tool_decision_recorded(
                        iterative_tool_decision("planned", call.name(), call.id()),
                    ));
                }
                IterativeLoopEvent::ToolObserved { observation } => {
                    let separately_executed = state.events().iter().any(|event| {
                        matches!(
                            event,
                            IterativeLoopEvent::ApprovalResolved { call_id, decision }
                                if call_id == observation.call_id() && decision == "approved"
                        )
                    });
                    events.push(SessionEvent::tool_decision_recorded(
                        iterative_tool_observation(
                            if observation.error().is_some() {
                                "failed"
                            } else {
                                "observed"
                            },
                            observation.tool_name(),
                            observation.call_id(),
                            canonical_tool_mutates(observation.tool_name()) && !separately_executed,
                        ),
                    ));
                    if let Some(evidence) = terminal_evidence(observation) {
                        events.push(SessionEvent::terminal_evidence_recorded(evidence));
                    }
                    events.push(SessionEvent::backend_response_received(
                        readable_observation(observation),
                    ));
                }
                _ => {}
            }
        }
        self.sessions.append_events(state.session_id(), &events);
    }

    fn remember_iterative_event_offset(&mut self, session_id: &str) {
        if let Some(state) = self.agent_iterative_states.get(session_id) {
            self.agent_iterative_event_offsets
                .insert(session_id.to_string(), state.events().len());
        }
    }

    pub(super) fn native_iterative_session_response(
        &self,
        session_id: &str,
        workspace_id: &str,
    ) -> ApiRouteResponse {
        ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
            self.sessions.get(session_id).as_ref(),
            workspace_id,
        ))
    }
}

fn terminal_evidence(observation: &ToolObservation) -> Option<TerminalEvidence> {
    matches!(
        observation.tool_name(),
        "desktoplab.run_terminal" | "desktoplab.run_tests"
    )
    .then(|| {
        let output = observation.output();
        let command = output
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("requested command");
        let stdout = output
            .get("stdout")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let stderr = output
            .get("stderr")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let combined = match (stdout.is_empty(), stderr.is_empty()) {
            (false, false) => format!("{stdout}\n{stderr}"),
            (false, true) => stdout.to_string(),
            (true, false) => stderr.to_string(),
            (true, true) => String::new(),
        };
        let exit_code = output
            .get("exitCode")
            .and_then(serde_json::Value::as_i64)
            .and_then(|code| i32::try_from(code).ok());
        TerminalEvidence::new(command, combined, exit_code)
    })
}

pub(super) fn iterative_tool_decision(state: &str, tool: &str, call_id: &str) -> String {
    format!("state={state} source=agent.iterative canonical={tool} tool={tool} call_id={call_id}")
}

fn iterative_tool_observation(state: &str, tool: &str, call_id: &str, mutation: bool) -> String {
    format!(
        "{} mutation={mutation}",
        iterative_tool_decision(state, tool, call_id)
    )
}

fn native_event_kind(state: Option<desktoplab_agent_session::SessionState>) -> &'static str {
    match state {
        Some(desktoplab_agent_session::SessionState::Completed) => "agent.step.completed",
        Some(desktoplab_agent_session::SessionState::Blocked) => "agent.step.blocked",
        Some(desktoplab_agent_session::SessionState::Running) => "agent.step.running",
        _ => "agent.step.failed",
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalApiRouter, iterative_tool_decision};

    #[test]
    fn every_product_backend_uses_the_canonical_iterative_loop() {
        let mut router = LocalApiRouter::default();
        router.complete_native_iterative_backend_sequence_for_test(["unused"]);

        for backend_id in [
            "backend.ollama",
            "backend.mlx-lm",
            "backend.lm-studio",
            "backend.high-end-local",
            "backend.codex",
        ] {
            assert!(
                router.backend_uses_native_iterative_loop(backend_id),
                "{backend_id} bypasses the canonical executor loop"
            );
        }
    }

    #[test]
    fn legacy_loop_requires_an_explicit_debug_harness() {
        let mut router = LocalApiRouter::default();

        assert!(!router.legacy_agent_test_harness_enabled);
        router.complete_agent_backend_for_test("fixture response");
        assert!(router.legacy_agent_test_harness_enabled);
        router.complete_native_iterative_backend_sequence_for_test(["unused"]);
        assert!(!router.legacy_agent_test_harness_enabled);
    }

    #[test]
    fn iterative_events_persist_structured_canonical_identity() {
        assert_eq!(
            iterative_tool_decision("observed", "desktoplab.patch_file", "call-7"),
            "state=observed source=agent.iterative canonical=desktoplab.patch_file tool=desktoplab.patch_file call_id=call-7"
        );
    }
}
