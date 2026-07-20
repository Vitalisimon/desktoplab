#[cfg(debug_assertions)]
use desktoplab_agent_engine::{AgentLoop, ApprovalDecision, LlmExecutionAdapter};
use desktoplab_agent_engine::{
    FirstPromptStep, IterativeToolCall, IterativeToolExecutor, ToolObservation,
};
use desktoplab_agent_session::SessionEvent;
#[cfg(debug_assertions)]
use desktoplab_agent_session::TerminalEvidence;
use desktoplab_backend_services::JobId;
use desktoplab_backend_services::{ApprovalState, BackendEventScope, EventReplayRequest, JobState};
use desktoplab_policy::PolicyEngine;
use desktoplab_runtime::{ProcessCommand, ProcessRunner, SystemProcessRunner};
#[cfg(debug_assertions)]
use desktoplab_tool_gateway::ToolGateway;
#[cfg(debug_assertions)]
use desktoplab_tool_gateway::{
    BatchPatchItem, BatchPatchOutcome, FilesystemApproval, FilesystemBatchPatchExecutor,
    FilesystemMutationExecutor, FilesystemMutationOutcome, FilesystemPatchApproval,
    FilesystemPatchExecutor, FilesystemPatchOutcome, FilesystemPatchRequest,
    FilesystemToolExecutor, FilesystemToolOutcome, ManagedProcessState, TerminalApproval,
    TerminalCommandRequest, TerminalExecutionStatus, TerminalToolExecutor, TerminalToolOutcome,
    TestRunApproval, TestRunOutcome, TestRunRequest, TestRunnerExecutor,
};
use desktoplab_tool_gateway::{
    GitToolExecutor, GitToolOutcome, SharedProcessRegistry, ToolIntent, WorkspacePathState,
    WorkspaceRoot,
};
use desktoplab_workspace::GitRepository;
#[cfg(debug_assertions)]
use desktoplab_workspace::{
    CommitApproval, CommitOperation, PushApproval, PushOperation, WorkspaceSearch,
    WorkspaceSearchLimits,
};
use serde_json::json;
#[cfg(debug_assertions)]
use std::fs;
use std::path::Path;
#[cfg(debug_assertions)]
use std::path::PathBuf;
#[cfg(debug_assertions)]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(debug_assertions)]
use super::AgentBackendExecutionMode;
use super::agent_attachments::{external_attachment_metadata, external_attachments};
#[cfg(debug_assertions)]
use super::agent_backend_recovery::{
    InitialBackendRecoveryState, LOCAL_BACKEND_TRANSPORT_ATTEMPTS, recover_initial_backend_output,
    retry_backend_transport,
};
use super::agent_pending::{
    PendingAgentAction, PendingAgentActionState, filesystem_mutation_postcondition_is_satisfied,
    pending_content_for_iterative_call,
};
#[cfg(debug_assertions)]
use super::agent_pending::{
    PendingMultiFilePatch, display_backend_response, has_structured_file_action,
    patch_postcondition_is_satisfied, pending_content_for_tool, pending_multi_file_patch_payload,
    pending_patch_payload, provider_output_recovery_evidence, structured_action_tool,
    structured_clarification_missing_blocked_action, structured_completion_message,
    unrecognized_tool_output_shape,
};
use super::dispatch::AgentContinuationMode;
use super::git_fingerprint::git_change_fingerprint;
#[cfg(debug_assertions)]
use super::helpers::body_string_array;
use super::helpers::{
    approval_json, body_bool_or, body_field, body_field_or, event_frame_json, job_state_value,
    query_value, segment,
};
use super::{ApiRouteResponse, LocalApiRouter, WorkspaceRecord};
use crate::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};

#[cfg(debug_assertions)]
fn display_agent_events(events: &[SessionEvent], hide_backend_response: bool) -> Vec<SessionEvent> {
    events
        .iter()
        .cloned()
        .filter_map(|event| match event {
            SessionEvent::BackendResponseReceived { .. } if hide_backend_response => None,
            SessionEvent::BackendResponseReceived { message } => {
                display_backend_response(&message).map(SessionEvent::backend_response_received)
            }
            other => Some(other),
        })
        .collect()
}

impl LocalApiRouter {
    pub(crate) fn create_session(
        &mut self,
        body: &str,
        continuation: AgentContinuationMode,
    ) -> ApiRouteResponse {
        if !self.setup.is_ready() {
            let workspace_id = body_field_or(body, "workspaceId", "");
            let prompt = body_field_or(body, "initialPrompt", "Inspect the repository");
            self.events.publish_json(
                BackendEventScope::Session,
                json!({
                    "kind":"agent.blocked",
                    "workspaceId":workspace_id,
                    "reason":"setup_not_ready",
                    "prompt":prompt
                }),
            );
            return ApiRouteResponse::ok(json!({
                "accepted":false,
                "sessionId":"session.blocked",
                "workspaceId":workspace_id,
                "executionBackendId":"backend.unavailable",
                "owner":"desktoplab",
                "state":"blocked",
                "plan":prompt,
                "summary":"Setup must finish before the agent can start.",
                "blockedReason":"setup_not_ready",
                "nextAction":"complete_setup",
                "timeline":[{
                    "sequence":1,
                    "kind":"blocked",
                    "message":"setup_not_ready",
                    "createdAt":current_timestamp()
                }]
            }));
        }
        let Some(workspace_id) =
            body_field(body, "workspaceId").filter(|workspace_id| !workspace_id.trim().is_empty())
        else {
            return workspace_not_selected_session_response(
                "",
                &self.selected_execution_backend_id(),
            );
        };
        let parent_session_id = body_field(body, "parentSessionId");
        let parent_session = parent_session_id
            .as_deref()
            .and_then(|session_id| self.sessions.get(session_id));
        if parent_session_id.is_some() && parent_session.is_none() {
            return ApiRouteResponse::bad_request(json!({
                "code":"PARENT_SESSION_NOT_FOUND",
                "message":"The parent session no longer exists."
            }));
        }
        if let Some(parent_id) = parent_session_id.as_deref()
            && self.sessions.workspace_id_for(parent_id).as_deref() != Some(&workspace_id)
        {
            return ApiRouteResponse::bad_request(json!({
                "code":"WORKSPACE_SESSION_MISMATCH",
                "message":"The child workspace must match its parent session."
            }));
        }
        let backend_id = parent_session.as_ref().map_or_else(
            || self.selected_execution_backend_id(),
            |parent| parent.execution_backend_id().to_string(),
        );
        let requested_backend_id = body_field_or(body, "executionBackendId", &backend_id);
        if requested_backend_id != backend_id {
            return ApiRouteResponse::bad_request(json!({
                "code":"ROUTE_BACKEND_MISMATCH",
                "message":"The requested execution backend does not match the selected route."
            }));
        }
        if self.workspace_record_for_id(&workspace_id).is_none() {
            return workspace_not_selected_session_response(&workspace_id, &backend_id);
        }
        if self.workspace_root_missing(&workspace_id) {
            return workspace_root_missing_session_response(&workspace_id, &backend_id);
        }
        let (agent_route_ready, blocked_reason) = self.selected_agent_route_readiness();
        if !agent_route_ready {
            let blocked_reason = blocked_reason.unwrap_or("execution_route_unavailable");
            return ApiRouteResponse::ok(json!({
                "accepted":false,
                "sessionId":"session.blocked",
                "workspaceId":workspace_id,
                "executionBackendId":backend_id,
                "owner":"desktoplab",
                "state":"blocked",
                "plan":body_field_or(body, "initialPrompt", "Inspect the repository"),
                "summary":"Choose an installed model with a verified agent protocol.",
                "blockedReason":blocked_reason,
                "nextAction":"choose_agent_model",
                "timeline":[{
                    "sequence":1,
                    "kind":"blocked",
                    "message":blocked_reason,
                    "createdAt":current_timestamp()
                }]
            }));
        }
        if has_planned_tool(body) {
            #[cfg(not(debug_assertions))]
            return planned_tool_test_harness_only_response();
            #[cfg(debug_assertions)]
            if !self.test_controls_enabled {
                return planned_tool_test_harness_only_response();
            }
        }
        if !body_bool_or(body, "newChat", false)
            && let Some(active_session_id) = self.pending_agent_session_for_workspace(&workspace_id)
        {
            return self.reject_agent_prompt_waiting_for_approval(
                &workspace_id,
                &backend_id,
                &active_session_id,
            );
        }
        let prompt = body_field_or(body, "initialPrompt", "Inspect the repository");
        let Some(_workspace) = self.workspace_record_for_id(&workspace_id) else {
            return workspace_root_missing_session_response(&workspace_id, &backend_id);
        };
        #[cfg(debug_assertions)]
        let workspace_root = Path::new(&_workspace.root_path);
        #[cfg(debug_assertions)]
        if body_field_or(body, "plannedTool", "") == "desktoplab.multi_file_refactor" {
            return self.create_multi_file_refactor_session(
                body,
                &workspace_id,
                &backend_id,
                &prompt,
            );
        }
        #[cfg(debug_assertions)]
        let tool_path = body_field_or(body, "toolPath", "README.md");
        #[cfg(debug_assertions)]
        if let Some(reason) = planned_tool_missing_reason(body) {
            let session = self.create_bound_agent_session(&workspace_id, &backend_id);
            self.inherit_agent_execution_binding(
                session.session_id(),
                parent_session_id.as_deref(),
            );
            self.agent_active_session_by_workspace
                .insert(workspace_id.clone(), session.session_id().to_string());
            self.persist_agent_active_sessions();
            return self.block_session_for_clarification_reason(
                session.session_id(),
                &workspace_id,
                &backend_id,
                &prompt,
                reason,
            );
        }
        let context_paths = context_paths(body);
        let external_attachments = match external_attachments(body) {
            Ok(attachments) => attachments,
            Err(error) => return ApiRouteResponse::bad_request(error),
        };
        #[cfg(debug_assertions)]
        let planned_tool = planned_tool(body, &tool_path);
        #[cfg(not(debug_assertions))]
        let planned_tool: Option<ToolIntent> = None;
        let expects_backend_file_action = false;
        let context_paths = context_paths_with_tool(&context_paths, planned_tool.as_ref());
        let provider_egress_approved = match self.provider_egress_approval_state(
            body,
            &workspace_id,
            &backend_id,
            &prompt,
            &context_paths,
            &external_attachments,
        ) {
            ProviderEgressState::Allowed => true,
            ProviderEgressState::NotNeeded => false,
            ProviderEgressState::Blocked(response) => return response,
        };
        let session = self.create_bound_agent_session(&workspace_id, &backend_id);
        self.inherit_agent_execution_binding(session.session_id(), parent_session_id.as_deref());
        self.agent_active_session_by_workspace
            .insert(workspace_id.clone(), session.session_id().to_string());
        self.persist_agent_active_sessions();
        self.events.publish_agent_event(
            "agent.prompt.accepted",
            &workspace_id,
            session.session_id(),
            session.execution_backend_id(),
            "Prompt accepted",
        );
        if body_bool_or(body, "stream", false) {
            self.agent_streaming_sessions
                .insert(session.session_id().to_string());
            self.events.publish_agent_event(
                "agent.stream.started",
                &workspace_id,
                session.session_id(),
                &backend_id,
                "Streaming response started",
            );
        }
        let prompt_tool = planned_tool.as_ref();
        let step = FirstPromptStep::new(
            session.session_id(),
            session.execution_backend_id(),
            &prompt,
        );
        let step = planned_tool
            .clone()
            .map_or(step.clone(), |tool| step.with_planned_tool(tool));
        let context = if is_external_backend(&backend_id) && !provider_egress_approved {
            None
        } else {
            self.safe_workspace_context(
                &workspace_id,
                &prompt,
                &context_paths,
                &external_attachments,
                None,
            )
        };
        if context.is_some() {
            self.events.publish_agent_event(
                "agent.context.read",
                &workspace_id,
                session.session_id(),
                session.execution_backend_id(),
                "Repository context read",
            );
        }
        if !context_paths.is_empty() {
            self.events.publish_json(
                BackendEventScope::Session,
                json!({
                    "kind":"agent.context.attached",
                    "workspaceId":workspace_id,
                    "sessionId":session.session_id(),
                    "executionBackendId":session.execution_backend_id(),
                    "paths":context_paths
                }),
            );
        }
        if !external_attachments.is_empty() {
            self.events.publish_json(
                BackendEventScope::Session,
                json!({
                    "kind":"agent.external_attachments.attached",
                    "workspaceId":workspace_id,
                    "sessionId":session.session_id(),
                    "executionBackendId":session.execution_backend_id(),
                    "attachments":external_attachment_metadata(&external_attachments),
                    "contentAttachments":external_attachments
                        .iter()
                        .filter(|attachment| attachment.get("contentText").is_some())
                        .count()
                }),
            );
        }
        let step = context.map_or(step.clone(), |context| step.with_context(context));
        let compiled_prompt = agent_backend_prompt(
            &step.compiled_prompt(),
            prompt_tool,
            expects_backend_file_action,
        );
        if self.backend_uses_native_iterative_loop(&backend_id) {
            let session_id = session.session_id().to_string();
            if continuation == AgentContinuationMode::Deferred
                || self.agent_streaming_sessions.contains(&session_id)
            {
                return self.queue_native_iterative_session(
                    &session_id,
                    &workspace_id,
                    &prompt,
                    &compiled_prompt,
                );
            }
            return self.start_native_iterative_session(
                &session_id,
                &workspace_id,
                &backend_id,
                &prompt,
                &compiled_prompt,
            );
        }
        #[cfg(not(debug_assertions))]
        {
            self.sessions
                .fail(session.session_id(), "native_agent_backend_required");
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(session.session_id()).as_ref(),
                &workspace_id,
            ));
        }
        #[cfg(debug_assertions)]
        {
            if !self.legacy_agent_test_harness_enabled {
                self.sessions
                    .fail(session.session_id(), "native_agent_backend_required");
                return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                    self.sessions.get(session.session_id()).as_ref(),
                    &workspace_id,
                ));
            }
            let (request, backend_adapter, initial_recovery) =
                self.agent_request_and_adapter(step, &backend_id, &compiled_prompt);
            let policy =
                PolicyEngine::default_conservative().with_approval_mode(self.session_approval_mode);
            let approval_decision = match self.agent_tool_approval_decision(
                body,
                session.session_id(),
                planned_tool.as_ref(),
            ) {
                Ok(decision) => decision,
                Err(error) => return ApiRouteResponse::state_journal_failed(error),
            };
            let mut loop_engine = AgentLoop::new(ToolGateway::new(policy))
                .with_backend_adapter(backend_adapter)
                .with_approval(approval_decision);
            let run = loop_engine.run(request);
            let backend_action_tool = if planned_tool.is_none() && !initial_recovery.exhausted() {
                structured_action_tool_from_session(run.session(), &workspace_id, workspace_root)
            } else {
                None
            };
            let backend_completion = run
                .session()
                .backend_responses()
                .last()
                .and_then(|response| structured_completion_message(response));
            let malformed_backend_action = initial_recovery.exhausted()
                || (planned_tool.is_none()
                    && backend_action_tool.is_none()
                    && backend_completion.is_none()
                    && run
                        .session()
                        .backend_responses()
                        .last()
                        .is_some_and(|response| looks_like_unrecognized_tool_output(response)));
            let missing_backend_file_action = expects_backend_file_action
                && backend_action_tool.is_none()
                && backend_completion.is_none();
            let approval_tool = planned_tool.as_ref().or(backend_action_tool.as_ref());
            let mut approval_request = if missing_backend_file_action || malformed_backend_action {
                AgentApprovalRequestOutcome::Malformed
            } else if run.pending_approvals() > 0 || backend_action_tool.is_some() {
                match self.request_agent_tool_approval(
                    session.session_id(),
                    approval_tool,
                    run.session(),
                    None,
                    &prompt,
                    workspace_root,
                ) {
                    Ok(outcome) => outcome,
                    Err(error) => return ApiRouteResponse::state_journal_failed(error),
                }
            } else {
                AgentApprovalRequestOutcome::Skipped
            };
            if approval_decision == ApprovalDecision::Approved {
                self.apply_approved_agent_tool(planned_tool.as_ref(), run.session());
            }
            let mut events = display_agent_events(
                run.events(),
                missing_backend_file_action || malformed_backend_action,
            );
            append_provider_output_recovery_event(run.events(), &mut events);
            if malformed_backend_action || missing_backend_file_action {
                if let Some(response) = run.session().backend_responses().last() {
                    events.push(SessionEvent::tool_decision_recorded(format!(
                        "provider_output_recovery:unrecognized_shape:{}",
                        unrecognized_tool_output_shape(response)
                    )));
                }
            }
            if initial_recovery.used() {
                events.push(SessionEvent::tool_decision_recorded(
                    "provider_output_recovery:initial_malformed_retry".to_string(),
                ));
            }
            if initial_recovery.exhausted() {
                events.push(SessionEvent::tool_decision_recorded(
                    "provider_output_recovery:protocol_exhausted".to_string(),
                ));
            }
            let observation_tool = planned_tool.as_ref().or(backend_action_tool.as_ref());
            let continue_after_observation = approval_request
                == AgentApprovalRequestOutcome::Skipped
                && tool_has_immediate_observation(observation_tool);
            if continue_after_observation
                || approval_request == AgentApprovalRequestOutcome::CheckpointBlocked
            {
                events.retain(|event| !matches!(event, SessionEvent::Completed { .. }));
            }
            self.sessions.append_events(session.session_id(), &events);
            if approval_request == AgentApprovalRequestOutcome::CheckpointBlocked {
                self.recover_initial_checkpoint_refusal(
                    session.session_id(),
                    &backend_id,
                    observation_tool,
                );
                approval_request = AgentApprovalRequestOutcome::Skipped;
            }
            match approval_request {
                AgentApprovalRequestOutcome::Malformed => self
                    .sessions
                    .fail(session.session_id(), "malformed structured file action"),
                AgentApprovalRequestOutcome::CheckpointBlocked => {
                    self.sessions
                        .block(session.session_id(), "checkpoint blocked risky mutation");
                }
                AgentApprovalRequestOutcome::Created
                | AgentApprovalRequestOutcome::Deduplicated => {
                    if run.session().state() != desktoplab_agent_session::SessionState::Blocked {
                        self.sessions
                            .block(session.session_id(), "waiting for approval");
                    }
                }
                AgentApprovalRequestOutcome::Skipped => {}
                AgentApprovalRequestOutcome::PersistenceFailed => self
                    .sessions
                    .fail(session.session_id(), "state_journal_failed"),
            }
            let run_state = run.session().state();
            if !matches!(
                run_state,
                desktoplab_agent_session::SessionState::Blocked
                    | desktoplab_agent_session::SessionState::Failed
            ) {
                self.append_filesystem_read_observation(session.session_id(), observation_tool);
                self.append_git_observation(session.session_id(), observation_tool);
                self.append_workspace_tool_observation(session.session_id(), observation_tool);
                if continue_after_observation
                    && self.session_can_continue_after_observation(session.session_id())
                {
                    self.finish_observation_continuation(
                        session.session_id(),
                        &backend_id,
                        observation_tool,
                        true,
                        None,
                    );
                }
            }
            let run_state = self
                .sessions
                .get(session.session_id())
                .map(|session| session.state())
                .unwrap_or(run_state);
            let (kind, message) = if tool_requests_clarification(backend_action_tool.as_ref()) {
                ("agent.step.blocked", "Action blocked")
            } else {
                match approval_request {
                    AgentApprovalRequestOutcome::Malformed => {
                        ("agent.step.failed", "Action failed")
                    }
                    AgentApprovalRequestOutcome::CheckpointBlocked
                    | AgentApprovalRequestOutcome::Created
                    | AgentApprovalRequestOutcome::Deduplicated => {
                        ("agent.step.blocked", "Action blocked")
                    }
                    AgentApprovalRequestOutcome::Skipped => match run_state {
                        desktoplab_agent_session::SessionState::Blocked => {
                            ("agent.step.blocked", "Action blocked")
                        }
                        desktoplab_agent_session::SessionState::Failed => {
                            ("agent.step.failed", "Action failed")
                        }
                        _ => ("agent.step.completed", "Response complete"),
                    },
                    AgentApprovalRequestOutcome::PersistenceFailed => {
                        ("agent.step.failed", "State journal failed")
                    }
                }
            };
            self.events.publish_agent_event(
                kind,
                &workspace_id,
                session.session_id(),
                session.execution_backend_id(),
                message,
            );
            ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(session.session_id()).as_ref(),
                &workspace_id,
            ))
        }
    }

    pub(crate) fn continue_session(
        &mut self,
        path: &str,
        body: &str,
        continuation: AgentContinuationMode,
    ) -> ApiRouteResponse {
        if !self.setup.is_ready() {
            return ApiRouteResponse::bad_request(json!({
                "code":"SETUP_REQUIRED",
                "message":"Setup must finish before the agent can continue.",
                "blockedReason":"setup_not_ready"
            }));
        }
        let session_id = segment(path, 2);
        let Some(existing_session) = self.sessions.get(&session_id) else {
            return ApiRouteResponse::not_found();
        };
        let Some(session_workspace_id) = self.sessions.workspace_id_for(&session_id) else {
            return ApiRouteResponse::not_found();
        };
        let workspace_id = body_field_or(body, "workspaceId", &session_workspace_id);
        if workspace_id != session_workspace_id {
            return ApiRouteResponse::bad_request(json!({
                "code":"WORKSPACE_SESSION_MISMATCH",
                "message":"The requested workspace does not own this session."
            }));
        }
        let backend_id = existing_session.execution_backend_id().to_string();
        if self.workspace_record_for_id(&workspace_id).is_none() {
            return workspace_not_selected_session_response(&workspace_id, &backend_id);
        }
        let selected_backend_id = self.selected_execution_backend_id();
        let requested_backend_id = body_field_or(body, "executionBackendId", &backend_id);
        if requested_backend_id != backend_id || selected_backend_id != backend_id {
            return ApiRouteResponse::bad_request(json!({
                "code":"ROUTE_BACKEND_MISMATCH",
                "message":"The requested execution backend does not match the session route."
            }));
        }
        if self.workspace_root_missing(&workspace_id) {
            return workspace_root_missing_session_response(&workspace_id, &backend_id);
        }
        let context_paths = context_paths(body);
        let external_attachments = match external_attachments(body) {
            Ok(attachments) => attachments,
            Err(error) => return ApiRouteResponse::bad_request(error),
        };
        let queued_replay = body_bool_or(body, "queuedReplay", false);
        if !queued_replay
            && matches!(
                existing_session.state(),
                desktoplab_agent_session::SessionState::Running
                    | desktoplab_agent_session::SessionState::Planning
            )
        {
            if !context_paths.is_empty() || !external_attachments.is_empty() {
                return ApiRouteResponse::bad_request(json!({
                    "code":"QUEUED_TURN_CONTEXT_UNSUPPORTED",
                    "message":"Wait for the current turn before sending attached context."
                }));
            }
            let prompt = body_field_or(body, "prompt", "Continue the repository work");
            self.sessions.enqueue_turn(&session_id, prompt);
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(&session_id).as_ref(),
                &workspace_id,
            ));
        }
        if has_planned_tool(body) {
            #[cfg(not(debug_assertions))]
            return planned_tool_test_harness_only_response();
            #[cfg(debug_assertions)]
            if !self.test_controls_enabled {
                return planned_tool_test_harness_only_response();
            }
        }
        self.agent_active_session_by_workspace
            .insert(workspace_id.clone(), session_id.clone());
        self.persist_agent_active_sessions();
        if let Some(approval_id) = body_field(body, "approvalId")
            && self
                .agent_pending_actions
                .get(&approval_id)
                .is_some_and(|action| action.session_id() == session_id)
        {
            return self.continue_pending_agent_action(&workspace_id, &backend_id, &approval_id);
        }
        if self.session_has_unresolved_pending_agent_action(&session_id) {
            return self.reject_agent_prompt_waiting_for_approval(
                &workspace_id,
                &backend_id,
                &session_id,
            );
        }
        let prompt = body_field_or(body, "prompt", "Continue the repository work");
        #[cfg(debug_assertions)]
        let Some(workspace) = self.execution_workspace_record(&session_id) else {
            return workspace_not_selected_session_response(&workspace_id, &backend_id);
        };
        #[cfg(debug_assertions)]
        let workspace_root = Path::new(&workspace.root_path);
        #[cfg(debug_assertions)]
        let tool_path = body_field_or(body, "toolPath", "README.md");
        #[cfg(debug_assertions)]
        if let Some(reason) = planned_tool_missing_reason(body) {
            return self.block_session_for_clarification_reason(
                &session_id,
                &workspace_id,
                &backend_id,
                &prompt,
                reason,
            );
        }
        #[cfg(debug_assertions)]
        let planned_tool = planned_tool(body, &tool_path);
        #[cfg(not(debug_assertions))]
        let planned_tool: Option<ToolIntent> = None;
        let expects_backend_file_action = false;
        let context_paths = context_paths_with_tool(&context_paths, planned_tool.as_ref());
        let provider_egress_approved = match self.provider_egress_approval_state(
            body,
            &workspace_id,
            &backend_id,
            &prompt,
            &context_paths,
            &external_attachments,
        ) {
            ProviderEgressState::Allowed => true,
            ProviderEgressState::NotNeeded => false,
            ProviderEgressState::Blocked(response) => return response,
        };
        self.events.publish_agent_event(
            "agent.prompt.accepted",
            &workspace_id,
            &session_id,
            &backend_id,
            "Prompt accepted",
        );
        let prompt_tool = planned_tool.as_ref();
        let step = FirstPromptStep::new(&session_id, &backend_id, &prompt);
        let step = planned_tool
            .clone()
            .map_or(step.clone(), |tool| step.with_planned_tool(tool));
        let context = if is_external_backend(&backend_id) && !provider_egress_approved {
            None
        } else {
            self.safe_workspace_context(
                &workspace_id,
                &prompt,
                &context_paths,
                &external_attachments,
                Some(&session_id),
            )
        };
        let step = context.map_or(step.clone(), |context| step.with_context(context));
        let compiled_prompt = agent_backend_prompt(
            &step.compiled_prompt(),
            prompt_tool,
            expects_backend_file_action,
        );
        if self.backend_uses_native_iterative_loop(&backend_id) {
            if continuation == AgentContinuationMode::Deferred {
                return self.queue_native_iterative_session(
                    &session_id,
                    &workspace_id,
                    &prompt,
                    &compiled_prompt,
                );
            }
            return self.start_native_iterative_session(
                &session_id,
                &workspace_id,
                &backend_id,
                &prompt,
                &compiled_prompt,
            );
        }
        #[cfg(not(debug_assertions))]
        {
            self.sessions
                .fail(&session_id, "native_agent_backend_required");
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(&session_id).as_ref(),
                &workspace_id,
            ));
        }
        #[cfg(debug_assertions)]
        {
            if !self.legacy_agent_test_harness_enabled {
                self.sessions
                    .fail(&session_id, "native_agent_backend_required");
                return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                    self.sessions.get(&session_id).as_ref(),
                    &workspace_id,
                ));
            }
            let (request, backend_adapter, initial_recovery) =
                self.agent_request_and_adapter(step, &backend_id, &compiled_prompt);
            let policy =
                PolicyEngine::default_conservative().with_approval_mode(self.session_approval_mode);
            let approval_decision =
                match self.agent_tool_approval_decision(body, &session_id, planned_tool.as_ref()) {
                    Ok(decision) => decision,
                    Err(error) => return ApiRouteResponse::state_journal_failed(error),
                };
            let mut loop_engine = AgentLoop::new(ToolGateway::new(policy))
                .with_backend_adapter(backend_adapter)
                .with_approval(approval_decision);
            let run = loop_engine.run(request);
            let backend_action_tool = if planned_tool.is_none() && !initial_recovery.exhausted() {
                structured_action_tool_from_session(run.session(), &workspace_id, workspace_root)
            } else {
                None
            };
            let backend_completion = run
                .session()
                .backend_responses()
                .last()
                .and_then(|response| structured_completion_message(response));
            let malformed_backend_action = initial_recovery.exhausted()
                || (planned_tool.is_none()
                    && backend_action_tool.is_none()
                    && backend_completion.is_none()
                    && run
                        .session()
                        .backend_responses()
                        .last()
                        .is_some_and(|response| looks_like_unrecognized_tool_output(response)));
            let missing_backend_file_action = expects_backend_file_action
                && backend_action_tool.is_none()
                && backend_completion.is_none();
            let approval_tool = planned_tool.as_ref().or(backend_action_tool.as_ref());
            let repeated_applied_action = backend_action_tool.as_ref().is_some_and(|tool| {
                run.session()
                    .backend_responses()
                    .last()
                    .is_some_and(|response| {
                        self.continuation_action_already_applied(&session_id, tool, response)
                    })
            });
            let mut approval_request = if missing_backend_file_action || malformed_backend_action {
                AgentApprovalRequestOutcome::Malformed
            } else if repeated_applied_action {
                AgentApprovalRequestOutcome::Skipped
            } else if run.pending_approvals() > 0 || backend_action_tool.is_some() {
                match self.request_agent_tool_approval(
                    &session_id,
                    approval_tool,
                    run.session(),
                    None,
                    &prompt,
                    workspace_root,
                ) {
                    Ok(outcome) => outcome,
                    Err(error) => return ApiRouteResponse::state_journal_failed(error),
                }
            } else {
                AgentApprovalRequestOutcome::Skipped
            };
            if approval_decision == ApprovalDecision::Approved {
                self.apply_approved_agent_tool(planned_tool.as_ref(), run.session());
            }
            let mut events = display_agent_events(
                run.events(),
                missing_backend_file_action || malformed_backend_action,
            );
            append_provider_output_recovery_event(run.events(), &mut events);
            if malformed_backend_action || missing_backend_file_action {
                if let Some(response) = run.session().backend_responses().last() {
                    events.push(SessionEvent::tool_decision_recorded(format!(
                        "provider_output_recovery:unrecognized_shape:{}",
                        unrecognized_tool_output_shape(response)
                    )));
                }
            }
            if initial_recovery.used() {
                events.push(SessionEvent::tool_decision_recorded(
                    "provider_output_recovery:initial_malformed_retry".to_string(),
                ));
            }
            if initial_recovery.exhausted() {
                events.push(SessionEvent::tool_decision_recorded(
                    "provider_output_recovery:protocol_exhausted".to_string(),
                ));
            }
            if repeated_applied_action {
                events.push(SessionEvent::tool_decision_recorded(
                    "provider_output_recovery:deduplicated_applied_action".to_string(),
                ));
            }
            let observation_tool = planned_tool.as_ref().or(backend_action_tool.as_ref());
            let continue_after_observation = approval_request
                == AgentApprovalRequestOutcome::Skipped
                && (tool_has_immediate_observation(observation_tool) || repeated_applied_action);
            if continue_after_observation
                || approval_request == AgentApprovalRequestOutcome::CheckpointBlocked
            {
                events.retain(|event| !matches!(event, SessionEvent::Completed { .. }));
            }
            self.sessions.append_events(&session_id, &events);
            if approval_request == AgentApprovalRequestOutcome::CheckpointBlocked {
                self.recover_initial_checkpoint_refusal(&session_id, &backend_id, observation_tool);
                approval_request = AgentApprovalRequestOutcome::Skipped;
            }
            match approval_request {
                AgentApprovalRequestOutcome::Malformed => self
                    .sessions
                    .fail(&session_id, "malformed structured file action"),
                AgentApprovalRequestOutcome::CheckpointBlocked => {
                    self.sessions
                        .block(&session_id, "checkpoint blocked risky mutation");
                }
                AgentApprovalRequestOutcome::Created
                | AgentApprovalRequestOutcome::Deduplicated => {
                    if run.session().state() != desktoplab_agent_session::SessionState::Blocked {
                        self.sessions.block(&session_id, "waiting for approval");
                    }
                }
                AgentApprovalRequestOutcome::Skipped => {}
                AgentApprovalRequestOutcome::PersistenceFailed => {
                    self.sessions.fail(&session_id, "state_journal_failed")
                }
            }
            let run_state = run.session().state();
            if !matches!(
                run_state,
                desktoplab_agent_session::SessionState::Blocked
                    | desktoplab_agent_session::SessionState::Failed
            ) {
                if repeated_applied_action {
                    if self.session_can_continue_after_observation(&session_id) {
                        let initial_observations = vec![
                        "The requested tool action was already applied in this session. Use the existing executor result and choose the next action required by the current user goal."
                            .to_string(),
                    ];
                        self.finish_observation_continuation(
                            &session_id,
                            &backend_id,
                            observation_tool,
                            false,
                            Some(initial_observations),
                        );
                    }
                } else {
                    self.append_filesystem_read_observation(&session_id, observation_tool);
                    self.append_git_observation(&session_id, observation_tool);
                    self.append_workspace_tool_observation(&session_id, observation_tool);
                    if continue_after_observation
                        && self.session_can_continue_after_observation(&session_id)
                    {
                        self.finish_observation_continuation(
                            &session_id,
                            &backend_id,
                            observation_tool,
                            true,
                            None,
                        );
                    }
                }
            }
            let run_state = self
                .sessions
                .get(&session_id)
                .map(|session| session.state())
                .unwrap_or(run_state);
            let (kind, message) = if tool_requests_clarification(backend_action_tool.as_ref()) {
                ("agent.step.blocked", "Action blocked")
            } else {
                match approval_request {
                    AgentApprovalRequestOutcome::Malformed => {
                        ("agent.step.failed", "Action failed")
                    }
                    AgentApprovalRequestOutcome::CheckpointBlocked
                    | AgentApprovalRequestOutcome::Created
                    | AgentApprovalRequestOutcome::Deduplicated => {
                        ("agent.step.blocked", "Action blocked")
                    }
                    AgentApprovalRequestOutcome::Skipped => match run_state {
                        desktoplab_agent_session::SessionState::Blocked => {
                            ("agent.step.blocked", "Action blocked")
                        }
                        desktoplab_agent_session::SessionState::Failed => {
                            ("agent.step.failed", "Action failed")
                        }
                        _ => ("agent.step.completed", "Response complete"),
                    },
                    AgentApprovalRequestOutcome::PersistenceFailed => {
                        ("agent.step.failed", "State journal failed")
                    }
                }
            };
            self.events
                .publish_agent_event(kind, &workspace_id, &session_id, &backend_id, message);
            ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(&session_id).as_ref(),
                &workspace_id,
            ))
        }
    }

    #[cfg(debug_assertions)]
    fn agent_tool_approval_decision(
        &mut self,
        body: &str,
        session_id: &str,
        tool: Option<&ToolIntent>,
    ) -> Result<ApprovalDecision, desktoplab_storage::StorageError> {
        let Some((action, operation_id)) = agent_tool_approval_key(tool) else {
            return Ok(ApprovalDecision::Pending);
        };
        let Some(approval_id) = body_field(body, "approvalId") else {
            return Ok(ApprovalDecision::Pending);
        };
        if self.consume_body_approved_record(body, session_id, &action, &operation_id, None)? {
            return Ok(ApprovalDecision::Approved);
        }
        if self.approvals.get(&approval_id).is_some_and(|record| {
            record.state() == ApprovalState::Denied
                && record.action() == action
                && record.operation_id() == operation_id
        }) {
            return Ok(ApprovalDecision::Denied);
        }
        Ok(ApprovalDecision::Pending)
    }

    fn pending_agent_session_for_workspace(&self, workspace_id: &str) -> Option<String> {
        self.agent_active_session_by_workspace
            .get(workspace_id)
            .filter(|session_id| self.session_has_unresolved_pending_agent_action(session_id))
            .cloned()
            .or_else(|| {
                self.sessions
                    .list_by_workspace(workspace_id)
                    .iter()
                    .rev()
                    .find(|session| {
                        self.session_has_unresolved_pending_agent_action(session.session_id())
                    })
                    .map(|session| session.session_id().to_string())
            })
    }

    fn session_has_unresolved_pending_agent_action(&self, session_id: &str) -> bool {
        self.agent_pending_actions.values().any(|action| {
            action.session_id() == session_id
                && action.state() == PendingAgentActionState::Pending
                && self
                    .approvals
                    .get(action.approval_id())
                    .is_some_and(|approval| {
                        matches!(
                            approval.state(),
                            ApprovalState::Pending | ApprovalState::Approved
                        ) && !approval.is_consumed()
                    })
        })
    }

    fn reject_agent_prompt_waiting_for_approval(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
        session_id: &str,
    ) -> ApiRouteResponse {
        self.agent_active_session_by_workspace
            .insert(workspace_id.to_string(), session_id.to_string());
        self.persist_agent_active_sessions();
        self.sessions
            .block(session_id, "session continuity pending user choice");
        self.events.publish_agent_event(
            "agent.prompt.blocked",
            workspace_id,
            session_id,
            backend_id,
            "Waiting for approval",
        );
        ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
            self.sessions.get(session_id).as_ref(),
            workspace_id,
        ))
    }

    #[cfg(debug_assertions)]
    fn block_session_for_clarification_reason(
        &mut self,
        session_id: &str,
        workspace_id: &str,
        backend_id: &str,
        prompt: &str,
        reason: &str,
    ) -> ApiRouteResponse {
        self.sessions.append_events(
            session_id,
            &[
                SessionEvent::planning_started(prompt),
                SessionEvent::blocked(reason),
            ],
        );
        self.events.publish_agent_event(
            "agent.step.blocked",
            workspace_id,
            session_id,
            backend_id,
            "Action blocked",
        );
        ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
            self.sessions.get(session_id).as_ref(),
            workspace_id,
        ))
    }

    pub(super) fn request_agent_tool_approval(
        &mut self,
        session_id: &str,
        tool: Option<&ToolIntent>,
        _session: &desktoplab_agent_session::AgentSession,
        _decision_response: Option<&str>,
        _prompt: &str,
        workspace_root: &Path,
    ) -> Result<AgentApprovalRequestOutcome, desktoplab_storage::StorageError> {
        let Some(tool) = tool else {
            return Ok(AgentApprovalRequestOutcome::Skipped);
        };
        let Some((action, operation_id)) = agent_tool_approval_key(Some(tool)) else {
            return Ok(AgentApprovalRequestOutcome::Skipped);
        };
        let iterative_call = self
            .agent_iterative_states
            .get(session_id)
            .and_then(|state| state.pending_approval())
            .map(|approval| approval.call().clone());
        let canonical_tool = iterative_call
            .as_ref()
            .map(IterativeToolCall::name)
            .unwrap_or("desktoplab.unknown")
            .to_string();
        #[cfg(debug_assertions)]
        let backend_response = iterative_call
            .is_none()
            .then(|| {
                _decision_response
                    .or_else(|| _session.backend_responses().last().map(String::as_str))
            })
            .flatten();
        #[cfg(debug_assertions)]
        if backend_response.is_some_and(|response| {
            provider_output_recovery_evidence(response)
                == Some("provider_output_recovery:invalid_json")
        }) && is_filesystem_mutation(tool)
        {
            return Ok(AgentApprovalRequestOutcome::Malformed);
        }
        let content = git_commit_pending_content(tool, workspace_root).or_else(|| {
            iterative_call
                .as_ref()
                .and_then(pending_content_for_iterative_call)
        });
        #[cfg(debug_assertions)]
        let content = content.or_else(|| {
            backend_response.and_then(|response| pending_content_for_tool(tool, response))
        });
        if git_commit_content_has_no_changes(tool, content.as_deref()) {
            self.sessions.append_events(
                session_id,
                &[SessionEvent::tool_decision_recorded(
                    "state=skipped source=agent.approval canonical=desktoplab.commit_changes tool=desktoplab.commit_changes reason=no_changes"
                        .to_string(),
                )],
            );
            return Ok(AgentApprovalRequestOutcome::Skipped);
        }
        #[cfg(debug_assertions)]
        if content.is_none()
            && backend_response.is_some_and(|response| has_structured_file_action(response))
        {
            return Ok(AgentApprovalRequestOutcome::Malformed);
        }
        let pending = PendingAgentAction::new(
            "approval.pending",
            session_id.to_string(),
            tool.clone(),
            content,
            is_filesystem_mutation(tool),
        );
        let pending = match iterative_call {
            Some(call) => pending.with_iterative_call(call),
            None => pending,
        };
        let pending = match approved_change_fingerprint_for_tool(tool, workspace_root) {
            Some(fingerprint) => pending.with_approved_change_fingerprint(fingerprint),
            None => pending,
        };
        let pending = match self.checkpoint_pending_agent_action(session_id, tool, pending) {
            CheckpointRequestOutcome::Ready(pending) => pending,
            CheckpointRequestOutcome::Blocked(reason) => {
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(format!(
                        "state=checkpoint_blocked reason={reason}"
                    ))],
                );
                return Ok(AgentApprovalRequestOutcome::CheckpointBlocked);
            }
        };
        if self.agent_pending_actions.values().any(|existing| {
            existing.session_id() == session_id
                && existing.payload_hash() == pending.payload_hash()
                && existing.state() == PendingAgentActionState::Pending
        }) {
            self.sessions.append_events(
                session_id,
                &[SessionEvent::tool_decision_recorded(format!(
                    "state=deduplicated source=agent.approval canonical={canonical_tool} tool={canonical_tool} reason=pending_approval_exists"
                ))],
            );
            return Ok(AgentApprovalRequestOutcome::Deduplicated);
        }
        let approvals_before = self.approvals.list();
        let pending_before = self.agent_pending_actions.clone();
        let approval = self.approvals.request_operation_with_payload_hash(
            session_id,
            action,
            operation_id,
            Some(pending.payload_hash().to_string()),
        );
        self.agent_pending_actions.insert(
            approval.id().to_string(),
            PendingAgentAction::new(
                approval.id().to_string(),
                session_id.to_string(),
                pending.tool().clone(),
                pending.content().map(ToString::to_string),
                pending.readback_after_write(),
            )
            .with_optional_approved_change_fingerprint(pending.approved_change_fingerprint())
            .with_optional_checkpoint(pending.checkpoint_id())
            .with_optional_iterative_call(pending.iterative_call()),
        );
        if let Err(error) = self.persist_agent_approval_journal() {
            self.approvals =
                desktoplab_backend_services::ApprovalService::from_records(approvals_before);
            self.agent_pending_actions = pending_before;
            return Err(error);
        }
        Ok(AgentApprovalRequestOutcome::Created)
    }

    fn checkpoint_pending_agent_action(
        &mut self,
        session_id: &str,
        tool: &ToolIntent,
        pending: PendingAgentAction,
    ) -> CheckpointRequestOutcome {
        if !requires_checkpoint_before_approval(tool) {
            return CheckpointRequestOutcome::Ready(pending);
        }
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            return CheckpointRequestOutcome::Blocked("execution_workspace_unavailable");
        };
        let checkpoint_id = format!("checkpoint.agent.{session_id}");
        let mut executor = GitToolExecutor::new(
            std::path::Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        match executor.prepare_checkpoint_ref(checkpoint_id.clone()) {
            GitToolOutcome::CheckpointReady(id) => {
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(format!(
                        "state=checkpoint_ready checkpoint={id} status=ready"
                    ))],
                );
                CheckpointRequestOutcome::Ready(pending.with_checkpoint(id, "ready"))
            }
            GitToolOutcome::Blocked(reason) => CheckpointRequestOutcome::Blocked(reason),
            _ => CheckpointRequestOutcome::Blocked("checkpoint_failed"),
        }
    }

    pub(super) fn continue_pending_agent_action(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
        approval_id: &str,
    ) -> ApiRouteResponse {
        self.continue_pending_agent_action_with_mode(workspace_id, backend_id, approval_id, false)
    }

    fn continue_pending_agent_action_with_mode(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
        approval_id: &str,
        defer_model: bool,
    ) -> ApiRouteResponse {
        let Some(pending) = self.agent_pending_actions.get(approval_id).cloned() else {
            return ApiRouteResponse::not_found();
        };
        let Some((action, operation_id)) = agent_tool_approval_key(Some(pending.tool())) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"AGENT_PENDING_ACTION_UNSUPPORTED",
                "message":"The pending agent action cannot be resumed."
            }));
        };
        let Some(approval) = self.approvals.get(approval_id) else {
            return ApiRouteResponse::not_found();
        };
        if pending.state() == PendingAgentActionState::Applied && approval.is_consumed() {
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }
        if approval.state() == ApprovalState::Denied {
            if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                pending.mark_failed();
            }
            if let Err(error) = self.persist_agent_approval_journal() {
                return ApiRouteResponse::state_journal_failed(error);
            }
            self.sessions.block(pending.session_id(), "approval denied");
            self.events.publish_agent_event(
                "agent.step.blocked",
                workspace_id,
                pending.session_id(),
                backend_id,
                "Action blocked",
            );
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }
        if approval.state() != ApprovalState::Approved {
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }
        let completed_by_worker = self.agent_completed_actions.contains_key(approval_id);
        if !approval.is_consumed()
            && !self.approvals.consume_approved_for_payload(
                approval_id,
                pending.session_id(),
                &action,
                &operation_id,
                Some(pending.payload_hash()),
            )
        {
            if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                pending.mark_failed();
            }
            if let Err(error) = self.persist_agent_approval_journal() {
                return ApiRouteResponse::state_journal_failed(error);
            }
            self.sessions
                .fail(pending.session_id(), "approval payload mismatch");
            self.events.publish_agent_event(
                "agent.step.failed",
                workspace_id,
                pending.session_id(),
                backend_id,
                "Action failed",
            );
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }

        if approval.is_consumed() && !completed_by_worker {
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }

        if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
            pending.mark_applying();
        }

        if let Err(error) = self.persist_agent_approval_journal() {
            return ApiRouteResponse::state_journal_failed(error);
        }

        let execution = self
            .agent_completed_actions
            .remove(approval_id)
            .unwrap_or_else(|| self.execute_approved_agent_action(&pending));
        if self
            .agent_iterative_states
            .contains_key(pending.session_id())
        {
            if defer_model {
                return self.defer_native_iterative_after_execution(
                    workspace_id,
                    backend_id,
                    &pending,
                    execution,
                );
            }
            return self.continue_native_iterative_after_execution(
                workspace_id,
                backend_id,
                &pending,
                execution,
            );
        }
        #[cfg(not(debug_assertions))]
        {
            if let Some(action) = self.agent_pending_actions.get_mut(approval_id) {
                action.mark_failed();
            }
            self.sessions
                .fail(pending.session_id(), "native_iterative_state_missing");
            return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ));
        }
        #[cfg(debug_assertions)]
        {
            match execution {
                PendingExecutionOutcome::Applied {
                    terminal_evidence,
                    response_evidence,
                } => {
                    let readback = self.read_pending_write_observation(&pending);
                    if let Some(Err(reason)) = readback {
                        if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                            pending.mark_failed();
                        }
                        if let Err(error) = self.persist_agent_approval_journal() {
                            return ApiRouteResponse::state_journal_failed(error);
                        }
                        self.sessions.append_events(
                            pending.session_id(),
                            &[
                                SessionEvent::resumed(),
                                SessionEvent::tool_decision_recorded(tool_decision_message(
                                    "approved",
                                    pending.tool(),
                                )),
                                SessionEvent::tool_decision_recorded(tool_decision_message(
                                    "executed",
                                    pending.tool(),
                                )),
                                SessionEvent::failed(reason),
                            ],
                        );
                        self.events.publish_agent_event(
                            "agent.step.failed",
                            workspace_id,
                            pending.session_id(),
                            backend_id,
                            "Action failed",
                        );
                        return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                            self.sessions.get(pending.session_id()).as_ref(),
                            workspace_id,
                        ));
                    }
                    if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                        pending.mark_applied();
                    }
                    if let Err(error) = self.persist_agent_approval_journal() {
                        self.sessions.fail(
                            pending.session_id(),
                            format!("state_journal_failed:{error}"),
                        );
                        return ApiRouteResponse::state_journal_failed(error);
                    }
                    if let ToolIntent::FilesystemWrite { path }
                    | ToolIntent::FilesystemPatch { path } = pending.tool()
                    {
                        self.agent_last_file_path_by_workspace
                            .insert(workspace_id.to_string(), path.to_string());
                    }
                    let mut continuation_observations = Vec::new();
                    let mut events = vec![
                        SessionEvent::resumed(),
                        SessionEvent::tool_decision_recorded(tool_decision_message(
                            "approved",
                            pending.tool(),
                        )),
                        SessionEvent::tool_decision_recorded(tool_decision_message(
                            "executed",
                            pending.tool(),
                        )),
                    ];
                    if let Some(Ok((tool, observation))) = readback {
                        continuation_observations.push(observation.clone());
                        events.push(SessionEvent::tool_decision_recorded(tool_decision_message(
                            "observed", &tool,
                        )));
                        events.push(SessionEvent::backend_response_received(observation));
                    }
                    if let Some(evidence) = terminal_evidence {
                        continuation_observations.push(format!(
                            "command={} exit_code={:?}\n{}",
                            evidence.command(),
                            evidence.exit_code(),
                            evidence.output()
                        ));
                        events.push(SessionEvent::terminal_evidence_recorded(evidence));
                    }
                    if let Some(response) = response_evidence {
                        continuation_observations.push(response.clone());
                        events.push(SessionEvent::backend_response_received(response));
                    }
                    for path in pending_changed_paths(&pending) {
                        events.push(SessionEvent::tool_decision_recorded(format!(
                            "context_index_updated path={path} mode=incremental"
                        )));
                    }
                    if continuation_observations.is_empty() {
                        continuation_observations.push("Tool completed successfully.".to_string());
                    }
                    self.sessions.append_events(pending.session_id(), &events);
                    self.finish_observation_continuation(
                        pending.session_id(),
                        backend_id,
                        Some(pending.tool()),
                        true,
                        Some(continuation_observations),
                    );
                    let continuation_state = self
                        .sessions
                        .get(pending.session_id())
                        .map(|session| session.state());
                    let (kind, message) = match continuation_state {
                        Some(desktoplab_agent_session::SessionState::Completed) => {
                            ("agent.step.completed", "Response complete")
                        }
                        Some(desktoplab_agent_session::SessionState::Blocked) => {
                            ("agent.step.blocked", "Action blocked")
                        }
                        _ => ("agent.step.failed", "Continuation failed"),
                    };
                    self.events.publish_agent_event(
                        kind,
                        workspace_id,
                        pending.session_id(),
                        backend_id,
                        message,
                    );
                }
                PendingExecutionOutcome::RecoverableFailure {
                    reason,
                    terminal_evidence,
                    response_evidence,
                } => {
                    if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                        pending.mark_failed();
                    }
                    if let Err(error) = self.persist_agent_approval_journal() {
                        self.sessions.fail(
                            pending.session_id(),
                            format!("state_journal_failed:{error}"),
                        );
                        return ApiRouteResponse::state_journal_failed(error);
                    }
                    let observation = format!(
                        "The approved tool action was not applied because the executor reported {reason}. Inspect current workspace state and choose a new canonical action."
                    );
                    let mut continuation_observations = vec![observation.clone()];
                    let mut events = vec![
                        SessionEvent::resumed(),
                        SessionEvent::tool_decision_recorded(tool_decision_message(
                            "failed",
                            pending.tool(),
                        )),
                        SessionEvent::backend_response_received(observation),
                    ];
                    if let Some(evidence) = terminal_evidence {
                        continuation_observations.push(format!(
                            "command={} exit_code={:?}\n{}",
                            evidence.command(),
                            evidence.exit_code(),
                            evidence.output()
                        ));
                        events.push(SessionEvent::terminal_evidence_recorded(evidence));
                    }
                    if let Some(response) = response_evidence {
                        continuation_observations.push(response.clone());
                        events.push(SessionEvent::backend_response_received(response));
                    }
                    if let Some(readback) = self.read_pending_write_observation(&pending) {
                        match readback {
                            Ok((read_tool, current_contents)) => {
                                events.push(SessionEvent::tool_decision_recorded(
                                    tool_decision_message("planned", &read_tool),
                                ));
                                events.push(SessionEvent::tool_decision_recorded(
                                    tool_decision_message("executed", &read_tool),
                                ));
                                events.push(SessionEvent::tool_decision_recorded(
                                    tool_decision_message("observed", &read_tool),
                                ));
                                events.push(SessionEvent::backend_response_received(
                                    current_contents.clone(),
                                ));
                                continuation_observations.push(current_contents);
                            }
                            Err(read_error) => {
                                let read_failure = format!(
                                    "The executor could not read the current write target after {reason}: {read_error}."
                                );
                                events.push(SessionEvent::backend_response_received(
                                    read_failure.clone(),
                                ));
                                continuation_observations.push(read_failure);
                            }
                        }
                    }
                    self.sessions.append_events(pending.session_id(), &events);
                    self.finish_observation_continuation(
                        pending.session_id(),
                        backend_id,
                        Some(pending.tool()),
                        false,
                        Some(continuation_observations),
                    );
                    let continuation_state = self
                        .sessions
                        .get(pending.session_id())
                        .map(|session| session.state());
                    let (kind, message) = match continuation_state {
                        Some(desktoplab_agent_session::SessionState::Completed) => {
                            ("agent.step.completed", "Response complete")
                        }
                        Some(desktoplab_agent_session::SessionState::Blocked) => {
                            ("agent.step.blocked", "Action blocked")
                        }
                        _ => ("agent.step.failed", "Continuation failed"),
                    };
                    self.events.publish_agent_event(
                        kind,
                        workspace_id,
                        pending.session_id(),
                        backend_id,
                        message,
                    );
                }
                PendingExecutionOutcome::Failed(reason) => {
                    if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
                        pending.mark_failed();
                    }
                    if let Err(error) = self.persist_agent_approval_journal() {
                        self.sessions.fail(
                            pending.session_id(),
                            format!("state_journal_failed:{error}"),
                        );
                        return ApiRouteResponse::state_journal_failed(error);
                    }
                    self.sessions.append_events(
                        pending.session_id(),
                        &[
                            SessionEvent::resumed(),
                            SessionEvent::tool_decision_recorded(tool_decision_message(
                                "failed",
                                pending.tool(),
                            )),
                            SessionEvent::failed(reason),
                        ],
                    );
                    self.events.publish_agent_event(
                        "agent.step.failed",
                        workspace_id,
                        pending.session_id(),
                        backend_id,
                        "Action failed",
                    );
                }
                PendingExecutionOutcome::NativeObservation(_) => self
                    .sessions
                    .fail(pending.session_id(), "native_observation_routing_mismatch"),
            }
            ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                self.sessions.get(pending.session_id()).as_ref(),
                workspace_id,
            ))
        }
    }

    #[cfg(debug_assertions)]
    fn execute_pending_agent_action(
        &self,
        pending: &PendingAgentAction,
    ) -> PendingExecutionOutcome {
        let Some(workspace) = self.execution_workspace_record(pending.session_id()) else {
            return PendingExecutionOutcome::Failed("execution_workspace_unavailable".to_string());
        };
        Self::execute_pending_agent_action_at(
            &workspace,
            pending,
            &self.agent_process_registry,
            &self.mcp_runtime,
        )
    }

    fn execute_approved_agent_action(
        &self,
        pending: &PendingAgentAction,
    ) -> PendingExecutionOutcome {
        let Some(persisted_call) = pending.iterative_call() else {
            #[cfg(not(debug_assertions))]
            return PendingExecutionOutcome::Failed(
                "legacy_pending_action_invalidated".to_string(),
            );
            #[cfg(debug_assertions)]
            {
                if !self.legacy_agent_test_harness_enabled {
                    return PendingExecutionOutcome::Failed(
                        "legacy_pending_action_invalidated".to_string(),
                    );
                }
                return self.execute_pending_agent_action(pending);
            }
        };
        let Some(call) = self.native_pending_call(pending) else {
            return PendingExecutionOutcome::NativeObservation(ToolObservation::failure(
                persisted_call,
                "iterative_state_missing",
            ));
        };
        let Some(workspace) = self.execution_workspace_record(pending.session_id()) else {
            return PendingExecutionOutcome::NativeObservation(ToolObservation::failure(
                &call,
                "execution_workspace_unavailable",
            ));
        };
        let approval_mode = self
            .agent_execution_bindings
            .get(pending.session_id())
            .map(|binding| binding.approval_mode())
            .unwrap_or_default();
        execute_native_approved_action_at(
            &workspace,
            pending,
            &call,
            approval_mode,
            &self.agent_process_registry,
            &self.mcp_runtime,
        )
    }

    fn native_pending_call(&self, pending: &PendingAgentAction) -> Option<IterativeToolCall> {
        let call = self
            .agent_iterative_states
            .get(pending.session_id())?
            .pending_approval()
            .map(|approval| approval.call().clone())?;
        (pending.iterative_call() == Some(&call)).then_some(call)
    }

    pub(crate) fn claim_next_approved_agent_action(&mut self) -> Option<ClaimedAgentAction> {
        if let Some((approval_id, session_id)) =
            self.agent_pending_actions.values().find_map(|pending| {
                let approval = self.approvals.get(pending.approval_id())?;
                (pending.state() == PendingAgentActionState::Pending
                    && approval.state() == ApprovalState::Approved
                    && !approval.is_consumed()
                    && self.sessions.get(pending.session_id()).is_none())
                .then(|| {
                    (
                        pending.approval_id().to_string(),
                        pending.session_id().to_string(),
                    )
                })
            })
        {
            self.fail_pending_execution_identity(
                &approval_id,
                &session_id,
                "execution_session_unavailable",
            );
            return None;
        }
        let approval_id = self.agent_pending_actions.values().find_map(|pending| {
            let approval = self.approvals.get(pending.approval_id())?;
            let session_state = self.sessions.get(pending.session_id())?.state();
            (pending.state() == PendingAgentActionState::Pending
                && approval.state() == ApprovalState::Approved
                && !approval.is_consumed())
            .then_some(session_state)
            .filter(|state| {
                !matches!(
                    state,
                    desktoplab_agent_session::SessionState::Paused
                        | desktoplab_agent_session::SessionState::Failed
                        | desktoplab_agent_session::SessionState::Cancelled
                        | desktoplab_agent_session::SessionState::Completed
                )
            })
            .map(|_| pending.approval_id().to_string())
        })?;
        let pending = self.agent_pending_actions.get(&approval_id)?.clone();
        if pending.iterative_call().is_none() && !self.legacy_agent_test_harness_enabled {
            self.agent_pending_actions
                .get_mut(&approval_id)?
                .mark_failed();
            self.sessions
                .fail(pending.session_id(), "legacy_pending_action_invalidated");
            if let Err(error) = self.persist_agent_approval_journal() {
                self.record_state_journal_result(Err(error));
            }
            return None;
        }
        let native_call = if pending.iterative_call().is_some() {
            match self.native_pending_call(&pending) {
                Some(call) => Some(call),
                None => {
                    self.agent_pending_actions
                        .get_mut(&approval_id)?
                        .mark_failed();
                    self.sessions
                        .fail(pending.session_id(), "iterative_state_missing");
                    if let Err(error) = self.persist_agent_approval_journal() {
                        self.record_state_journal_result(Err(error));
                    }
                    return None;
                }
            }
        } else {
            None
        };
        let session_id = pending.session_id().to_string();
        let Some(session) = self.sessions.get(&session_id) else {
            self.fail_pending_execution_identity(
                &approval_id,
                &session_id,
                "execution_session_unavailable",
            );
            return None;
        };
        let backend_id = session.execution_backend_id().to_string();
        let Some(workspace) = self.execution_workspace_record(&session_id) else {
            self.fail_pending_execution_identity(
                &approval_id,
                &session_id,
                "execution_workspace_unavailable",
            );
            return None;
        };
        let (action, operation_id) = agent_tool_approval_key(Some(pending.tool()))?;
        if !self.approvals.consume_approved_for_payload(
            &approval_id,
            pending.session_id(),
            &action,
            &operation_id,
            Some(pending.payload_hash()),
        ) {
            return None;
        }
        self.agent_pending_actions
            .get_mut(&approval_id)?
            .mark_applying();
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
            return None;
        }
        let workspace_id = workspace.workspace_id.clone();
        let approval_mode = self
            .agent_execution_bindings
            .get(&session_id)
            .map(|binding| binding.approval_mode())
            .unwrap_or_default();
        Some(ClaimedAgentAction {
            approval_id,
            workspace_id,
            backend_id,
            workspace,
            process_registry: self.agent_process_registry.clone(),
            mcp_runtime: self.mcp_runtime.clone(),
            pending,
            native_call,
            approval_mode,
        })
    }

    pub(crate) fn fail_pending_execution_identity(
        &mut self,
        approval_id: &str,
        session_id: &str,
        reason: &str,
    ) {
        if let Some(pending) = self.agent_pending_actions.get_mut(approval_id) {
            pending.mark_failed();
        }
        self.sessions.fail(session_id, reason);
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
        }
    }

    pub(crate) fn complete_claimed_agent_action_deferred(
        &mut self,
        completed: CompletedAgentAction,
    ) {
        self.complete_claimed_agent_action_with_mode(completed, true);
    }

    fn complete_claimed_agent_action_with_mode(
        &mut self,
        completed: CompletedAgentAction,
        defer_model: bool,
    ) {
        self.agent_completed_actions
            .insert(completed.approval_id.clone(), completed.outcome);
        let _ = self.continue_pending_agent_action_with_mode(
            &completed.workspace_id,
            &completed.backend_id,
            &completed.approval_id,
            defer_model,
        );
    }

    #[cfg(debug_assertions)]
    fn execute_pending_agent_action_at(
        workspace: &WorkspaceRecord,
        pending: &PendingAgentAction,
        process_registry: &SharedProcessRegistry,
        mcp_runtime: &desktoplab_tool_gateway::SharedMcpRuntime,
    ) -> PendingExecutionOutcome {
        match pending.tool() {
            ToolIntent::FilesystemWrite { path } | ToolIntent::FilesystemPatch { path } => {
                let Some(contents) = pending.content() else {
                    return PendingExecutionOutcome::Failed("missing pending content".to_string());
                };
                if let Some(files) = pending_multi_file_patch_payload(contents) {
                    return execute_multi_file_patch(Path::new(&workspace.root_path), &files);
                }
                if let Some((expected, replacement)) = pending_patch_payload(contents) {
                    let mut executor = FilesystemPatchExecutor::new(
                        std::path::Path::new(&workspace.root_path),
                        PolicyEngine::default_conservative(),
                    );
                    return match executor.apply(
                        FilesystemPatchRequest::replace(path, expected, replacement),
                        FilesystemPatchApproval::Approved,
                    ) {
                        FilesystemPatchOutcome::Patched(evidence) => {
                            PendingExecutionOutcome::Applied {
                                terminal_evidence: None,
                                response_evidence: Some(format!(
                                    "{}{}",
                                    evidence.before_diff(),
                                    evidence.after_diff()
                                )),
                            }
                        }
                        FilesystemPatchOutcome::ApprovalRequired => {
                            PendingExecutionOutcome::Failed(
                                "filesystem patch approval was not applied".to_string(),
                            )
                        }
                        FilesystemPatchOutcome::Denied => {
                            PendingExecutionOutcome::Failed("filesystem patch denied".to_string())
                        }
                        FilesystemPatchOutcome::Blocked("patch_conflict") => {
                            PendingExecutionOutcome::recoverable("patch_conflict")
                        }
                        FilesystemPatchOutcome::Blocked(reason) => {
                            PendingExecutionOutcome::Failed(reason.to_string())
                        }
                    };
                }
                let mut executor = FilesystemToolExecutor::new(
                    std::path::Path::new(&workspace.root_path),
                    PolicyEngine::default_conservative(),
                );
                match executor.write(path, contents, FilesystemApproval::Approved) {
                    FilesystemToolOutcome::Written => PendingExecutionOutcome::Applied {
                        terminal_evidence: None,
                        response_evidence: None,
                    },
                    FilesystemToolOutcome::Unchanged => {
                        PendingExecutionOutcome::recoverable("write_no_change")
                    }
                    FilesystemToolOutcome::ApprovalRequired => PendingExecutionOutcome::Failed(
                        "filesystem approval was not applied".to_string(),
                    ),
                    FilesystemToolOutcome::Denied => {
                        PendingExecutionOutcome::Failed("filesystem write denied".to_string())
                    }
                    FilesystemToolOutcome::Blocked(reason) => {
                        PendingExecutionOutcome::Failed(reason.to_string())
                    }
                    FilesystemToolOutcome::Read(_) => {
                        PendingExecutionOutcome::Failed("unexpected filesystem read".to_string())
                    }
                }
            }
            ToolIntent::FilesystemCreateDirectory { .. }
            | ToolIntent::FilesystemMove { .. }
            | ToolIntent::FilesystemDelete { .. } => {
                execute_pending_filesystem_mutation(Path::new(&workspace.root_path), pending.tool())
            }
            ToolIntent::Terminal {
                workspace_id,
                working_directory,
                command,
                risk_class,
            } => {
                let mut executor = TerminalToolExecutor::new(
                    std::path::Path::new(&workspace.root_path),
                    PolicyEngine::default_conservative(),
                    Duration::from_secs(30),
                    64 * 1024,
                );
                let workspace_scope = workspace_id
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(&workspace.workspace_id);
                let request = TerminalCommandRequest::for_workspace(workspace_scope, command)
                    .with_working_directory(working_directory)
                    .with_risk_class(*risk_class);
                let started_at = Instant::now();
                match executor.execute(request, TerminalApproval::Approved) {
                    TerminalToolOutcome::Completed(result) => {
                        let evidence = terminal_evidence(
                            command,
                            working_directory,
                            started_at.elapsed().as_millis(),
                            &result,
                        );
                        match execution_status_failure(&result.status(), false) {
                            Some(reason) => PendingExecutionOutcome::execution_failure(
                                reason,
                                Some(evidence),
                                None,
                            ),
                            None => PendingExecutionOutcome::Applied {
                                terminal_evidence: Some(evidence),
                                response_evidence: None,
                            },
                        }
                    }
                    TerminalToolOutcome::ApprovalRequired => PendingExecutionOutcome::Failed(
                        "terminal approval was not applied".to_string(),
                    ),
                    TerminalToolOutcome::Denied => {
                        PendingExecutionOutcome::Failed("terminal command denied".to_string())
                    }
                    TerminalToolOutcome::Blocked(reason) => {
                        PendingExecutionOutcome::Failed(reason.to_string())
                    }
                }
            }
            ToolIntent::ProcessStart {
                working_directory,
                command,
                ..
            } => match process_registry.start(
                Path::new(&workspace.root_path),
                &workspace.workspace_id,
                pending.session_id(),
                command,
                working_directory,
            ) {
                Ok(snapshot) => {
                    let (status, exit_code) = match snapshot.state() {
                        ManagedProcessState::Running => ("running", None),
                        ManagedProcessState::Exited(code) => ("exited", Some(*code)),
                        ManagedProcessState::Killed => ("killed", None),
                    };
                    PendingExecutionOutcome::Applied {
                        terminal_evidence: None,
                        response_evidence: Some(
                            json!({
                                "processId":snapshot.process_id(),
                                "status":status,
                                "exitCode":exit_code,
                                "stdout":snapshot.stdout(),
                                "stderr":snapshot.stderr()
                            })
                            .to_string(),
                        ),
                    }
                }
                Err(reason) => PendingExecutionOutcome::Failed(reason),
            },
            ToolIntent::TestRun {
                workspace_id,
                working_directory,
                command,
                reason,
            } => {
                let workspace_scope = workspace_id
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(&workspace.workspace_id);
                let request = TestRunRequest::new(workspace_scope, command, reason)
                    .with_working_directory(working_directory);
                let mut executor = TestRunnerExecutor::new(
                    std::path::Path::new(&workspace.root_path),
                    PolicyEngine::default_conservative(),
                    Duration::from_secs(30),
                    64 * 1024,
                );
                match executor.run(request, TestRunApproval::Approved) {
                    TestRunOutcome::Completed(evidence) => {
                        let terminal = test_terminal_evidence(&evidence, &workspace.root_path);
                        let response =
                            workspace_relative_evidence(&evidence.summary(), &workspace.root_path);
                        match execution_status_failure(&evidence.status(), true) {
                            Some(reason) => PendingExecutionOutcome::execution_failure(
                                reason,
                                Some(terminal),
                                Some(response),
                            ),
                            None => PendingExecutionOutcome::Applied {
                                terminal_evidence: Some(terminal),
                                response_evidence: Some(response),
                            },
                        }
                    }
                    TestRunOutcome::ApprovalRequired => {
                        PendingExecutionOutcome::Failed("test approval was not applied".to_string())
                    }
                    TestRunOutcome::Denied => {
                        PendingExecutionOutcome::Failed("test run denied".to_string())
                    }
                    TestRunOutcome::Blocked(reason) => {
                        PendingExecutionOutcome::Failed(reason.to_string())
                    }
                }
            }
            ToolIntent::GitCommit { message, .. } => {
                let root = std::path::Path::new(&workspace.root_path);
                let Ok(repo) = GitRepository::open(root) else {
                    return PendingExecutionOutcome::Failed("git_repository_required".to_string());
                };
                let status = repo.status().ok();
                let status_entries = status
                    .as_ref()
                    .map(|status| status.entries().to_vec())
                    .unwrap_or_default();
                let diff_text = repo
                    .diff()
                    .map(|diff| diff.as_text().to_string())
                    .unwrap_or_default();
                let current_fingerprint = git_change_fingerprint(&status_entries, &diff_text);
                let changed_files = status
                    .as_ref()
                    .map(|status| {
                        status
                            .files()
                            .iter()
                            .map(|file| file.path().to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let Some(approved_changed_files) =
                    pending_git_commit_changed_files(pending.content())
                else {
                    return PendingExecutionOutcome::Failed(
                        "missing_reviewed_file_set".to_string(),
                    );
                };
                match pending.approved_change_fingerprint() {
                    Some(approved) if approved == current_fingerprint => {}
                    Some(_) => {
                        return PendingExecutionOutcome::Failed(
                            "working_tree_changed_after_approval".to_string(),
                        );
                    }
                    None => {
                        return PendingExecutionOutcome::Failed(
                            "missing_approved_change_fingerprint".to_string(),
                        );
                    }
                }
                if !approved_changed_files
                    .iter()
                    .all(|file| changed_files.contains(file))
                {
                    return PendingExecutionOutcome::Failed(
                        "working_tree_changed_after_approval".to_string(),
                    );
                }
                match CommitOperation::new(CommitApproval::Approved).commit(
                    root,
                    pending.session_id(),
                    message,
                    &approved_changed_files,
                ) {
                    Ok(outcome) if outcome.status() == "committed" => {
                        PendingExecutionOutcome::Applied {
                            terminal_evidence: None,
                            response_evidence: Some(format!("Git commit created: {message}")),
                        }
                    }
                    Ok(outcome) => PendingExecutionOutcome::Failed(outcome.status().to_string()),
                    Err(error) => PendingExecutionOutcome::Failed(error.to_string()),
                }
            }
            ToolIntent::GitPush { remote, branch } => {
                let root = std::path::Path::new(&workspace.root_path);
                match PushOperation::new(PushApproval::Approved).push(root, remote, branch) {
                    Ok(outcome) if outcome.status() == "pushed" => {
                        PendingExecutionOutcome::Applied {
                            terminal_evidence: None,
                            response_evidence: Some(format!(
                                "Git push completed: {remote} {branch}"
                            )),
                        }
                    }
                    Ok(outcome) => PendingExecutionOutcome::Failed(outcome.status().to_string()),
                    Err(error) => PendingExecutionOutcome::Failed(error.to_string()),
                }
            }
            ToolIntent::McpInvoke { tool_id, arguments } => {
                match mcp_runtime.invoke_with_tokens(
                    tool_id,
                    arguments.clone(),
                    true,
                    &mut crate::mcp_tokens::NativeMcpTokenSource,
                ) {
                    Ok(response) => PendingExecutionOutcome::Applied {
                        terminal_evidence: None,
                        response_evidence: Some(response.to_string()),
                    },
                    Err(error) => PendingExecutionOutcome::Failed(error),
                }
            }
            _ => PendingExecutionOutcome::Failed("unsupported pending action".to_string()),
        }
    }

    #[cfg(debug_assertions)]
    fn append_filesystem_read_observation(&mut self, session_id: &str, tool: Option<&ToolIntent>) {
        let Some(ToolIntent::FilesystemRead { path }) = tool else {
            return;
        };
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            self.sessions
                .fail(session_id, "execution_workspace_unavailable");
            return;
        };
        let mut executor = FilesystemToolExecutor::new(
            std::path::Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        let read_tool = ToolIntent::filesystem_read(path);
        match executor.read(path) {
            FilesystemToolOutcome::Read(contents) => {
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(tool_decision_message(
                            "observed", &read_tool,
                        )),
                        SessionEvent::backend_response_received(format!(
                            "Read {path}:\n{contents}"
                        )),
                    ],
                );
            }
            FilesystemToolOutcome::Blocked(reason) => {
                self.append_executor_failure_observation(session_id, &read_tool, reason);
            }
            FilesystemToolOutcome::ApprovalRequired => {
                self.sessions
                    .fail(session_id, "filesystem read unexpectedly required approval");
            }
            FilesystemToolOutcome::Denied => {
                self.sessions.fail(session_id, "filesystem read denied");
            }
            FilesystemToolOutcome::Written => {
                self.sessions
                    .fail(session_id, "unexpected filesystem write observation");
            }
            FilesystemToolOutcome::Unchanged => {
                self.sessions
                    .fail(session_id, "unexpected unchanged filesystem observation");
            }
        }
    }

    #[cfg(debug_assertions)]
    fn append_git_observation(&mut self, session_id: &str, tool: Option<&ToolIntent>) {
        let Some(tool) = tool else {
            return;
        };
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            self.sessions
                .fail(session_id, "execution_workspace_unavailable");
            return;
        };
        let mut executor = GitToolExecutor::new(
            std::path::Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        let observation = match tool {
            ToolIntent::GitStatus => executor
                .status_observation()
                .unwrap_or_else(|reason| format!("Git status blocked: {reason}")),
            ToolIntent::GitDiff { .. } => executor
                .diff_observation()
                .unwrap_or_else(|reason| format!("Git diff blocked: {reason}")),
            _ => return,
        };
        self.sessions.append_events(
            session_id,
            &[
                SessionEvent::tool_decision_recorded(tool_decision_message("observed", tool)),
                SessionEvent::backend_response_received(observation),
            ],
        );
    }

    #[cfg(debug_assertions)]
    fn append_workspace_tool_observation(&mut self, session_id: &str, tool: Option<&ToolIntent>) {
        let Some(tool) = tool else {
            return;
        };
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            self.sessions
                .fail(session_id, "execution_workspace_unavailable");
            return;
        };
        let root = std::path::Path::new(&workspace.root_path);
        let observation = match tool {
            ToolIntent::FilesystemList { path } => {
                workspace_file_list_observation(root, path.as_deref())
            }
            ToolIntent::SearchText { query, path } => {
                workspace_search_observation(root, query, path.as_deref())
            }
            ToolIntent::CreateCheckpoint { label } => {
                self.workspace_checkpoint_observation(root, session_id, label)
            }
            ToolIntent::Clarify { question, .. } => {
                let reason = format!("clarification_required:{question}");
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(tool_decision_message(
                            "observed", tool,
                        )),
                        SessionEvent::blocked(reason),
                    ],
                );
                return;
            }
            _ => return,
        };
        match observation {
            Ok(message) => self.sessions.append_events(
                session_id,
                &[
                    SessionEvent::tool_decision_recorded(tool_decision_message("observed", tool)),
                    SessionEvent::backend_response_received(message),
                ],
            ),
            Err(reason) => self.append_executor_failure_observation(session_id, tool, &reason),
        }
    }

    #[cfg(debug_assertions)]
    fn append_executor_failure_observation(
        &mut self,
        session_id: &str,
        tool: &ToolIntent,
        reason: &str,
    ) {
        self.sessions.append_events(
            session_id,
            &[
                SessionEvent::tool_decision_recorded(tool_decision_message("failed", tool)),
                SessionEvent::backend_response_received(format!(
                    "Observation: tool {} failed safely with executor_reason={reason}. No action was applied and no content was returned. Choose a materially different canonical action or complete with a clear user-facing explanation.",
                    tool_evidence(tool)
                )),
            ],
        );
    }

    #[cfg(debug_assertions)]
    fn finish_observation_continuation(
        &mut self,
        session_id: &str,
        backend_id: &str,
        tool: Option<&ToolIntent>,
        action_applied: bool,
        initial_observations: Option<Vec<String>>,
    ) {
        let Some(tool) = tool else {
            return;
        };
        let mut tool_summary = format!("{tool:?}");
        let user_goal = self
            .sessions
            .get(session_id)
            .and_then(|session| session.plans().last().cloned())
            .unwrap_or_else(|| "Complete the current repository task.".to_string());
        let mut observations =
            initial_observations.unwrap_or_else(|| vec![self.latest_agent_observation(session_id)]);
        let mut previous_immediate_tool =
            tool_has_immediate_observation(Some(tool)).then(|| tool.clone());
        let approved_action_completed =
            action_applied && !tool_has_immediate_observation(Some(tool));
        let mut malformed_recovery_used = false;
        let mut no_progress_recovery_used = false;
        let mut final_read_only_recovery_used = false;
        if approved_action_completed
            && is_filesystem_mutation(tool)
            && self.session_has_unresolved_failed_validation(session_id)
            && self.pause_for_automatic_validation_rerun(session_id)
        {
            return;
        }
        for _ in 0..12 {
            let response = match self.continue_agent_after_approved_tool(
                backend_id,
                &user_goal,
                &tool_summary,
                &observations,
            ) {
                Ok(response) => response,
                Err(reason) => {
                    self.sessions.fail(session_id, reason);
                    return;
                }
            };
            if let Some(message) = structured_completion_message(&response) {
                if self.session_has_unresolved_failed_validation(session_id) {
                    self.sessions.append_events(
                        session_id,
                        &[SessionEvent::tool_decision_recorded(
                            "provider_output_recovery:validation_still_failing".to_string(),
                        )],
                    );
                    tool_summary =
                        "Resolve the latest failed validation before completion".to_string();
                    let next_step = if action_applied && is_filesystem_mutation(tool) {
                        if self.pause_for_automatic_validation_rerun(session_id) {
                            return;
                        }
                        "The requested filesystem mutation was already applied and read back successfully. Do not propose another mutation before validation. Call desktoplab.run_tests with the concrete validation command already established by the task."
                    } else {
                        "Inspect or repair as needed, then call desktoplab.run_tests with a concrete command."
                    };
                    observations.push(format!(
                        "The latest executor-owned validation is still failing. Do not claim completion. {next_step} Completion is allowed only after a later passing validation observation."
                    ));
                    continue;
                }
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::backend_response_received(message),
                        SessionEvent::completed("agent loop completed"),
                    ],
                );
                return;
            }
            let Some(next_tool) = structured_action_tool(&response) else {
                if provider_output_requires_initial_retry(&response) {
                    if !malformed_recovery_used {
                        malformed_recovery_used = true;
                        tool_summary =
                            "Recover malformed provider output after executor observation"
                                .to_string();
                        observations = vec![format!(
                            "A tool observation is available, but the previous provider output was not a valid canonical tool call. Use desktoplab.complete with a concise grounded message when the goal is complete, or emit one valid canonical tool call when more work is required. Executor evidence:\n{}",
                            observations.join("\n\n")
                        )];
                        continue;
                    }
                    if is_filesystem_mutation(tool)
                        && approved_action_completed
                        && previous_immediate_tool.is_some()
                    {
                        self.sessions.append_events(
                            session_id,
                            &[
                                SessionEvent::tool_decision_recorded(
                                    "provider_output_recovery:executor_grounded_completion"
                                        .to_string(),
                                ),
                                SessionEvent::completed(
                                    "agent loop completed from verified executor evidence",
                                ),
                            ],
                        );
                        return;
                    }
                    self.sessions.append_events(
                        session_id,
                        &[SessionEvent::tool_decision_recorded(format!(
                            "provider_output_recovery:unrecognized_shape:{}",
                            unrecognized_tool_output_shape(&response)
                        ))],
                    );
                    self.sessions
                        .fail(session_id, "malformed structured file action");
                    return;
                }
                if action_applied
                    && is_filesystem_mutation(tool)
                    && self.session_has_unresolved_failed_validation(session_id)
                    && self.pause_for_automatic_validation_rerun(session_id)
                {
                    return;
                }
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::backend_response_received(response),
                        SessionEvent::completed("agent loop completed"),
                    ],
                );
                return;
            };
            // A canonical tool call proves the model recovered. Allow one fresh
            // format correction after later executor observations.
            malformed_recovery_used = false;
            if approved_action_completed
                && is_filesystem_mutation(tool)
                && self.session_has_unresolved_failed_validation(session_id)
                && is_filesystem_mutation(&next_tool)
            {
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:duplicate_mutation_before_validation".to_string(),
                    )],
                );
                if self.pause_for_automatic_validation_rerun(session_id) {
                    return;
                }
            }
            if !approved_action_completed
                && !no_progress_recovery_used
                && clarification_targets_approval_gated_action(&next_tool)
            {
                no_progress_recovery_used = true;
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:approval_is_confirmation".to_string(),
                    )],
                );
                tool_summary =
                    "Propose the blocked action so DesktopLab can request approval".to_string();
                observations = vec![format!(
                    "DesktopLab approval is the confirmation boundary for the declared blocked action. Propose that canonical action now when its required arguments are available. Repeat clarification only when a concrete required argument is missing. Existing executor evidence:\n{}",
                    observations.join("\n\n")
                )];
                continue;
            }
            if !approved_action_completed
                && clarification_repeats_observed_read(
                    &next_tool,
                    previous_immediate_tool.as_ref().unwrap_or(tool),
                )
            {
                if no_progress_recovery_used {
                    self.sessions
                        .fail(session_id, "agent_no_progress_after_read_only_action");
                    return;
                }
                no_progress_recovery_used = true;
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:observed_read_not_blocking".to_string(),
                    )],
                );
                tool_summary = "Synthesize from completed read-only evidence".to_string();
                observations = vec![format!(
                    "The declared read-only blocked action already completed and its executor evidence is available. Use desktoplab.complete with a concise grounded message when that evidence satisfies the user goal; otherwise select a different canonical action. Existing executor evidence:\n{}",
                    observations.join("\n\n")
                )];
                continue;
            }
            if !approved_action_completed
                && matches!(
                    next_tool,
                    ToolIntent::Clarify {
                        blocked_action: None,
                        ..
                    }
                )
            {
                if no_progress_recovery_used {
                    self.sessions
                        .fail(session_id, "agent_no_progress_after_read_only_action");
                    return;
                }
                no_progress_recovery_used = true;
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:read_only_no_progress".to_string(),
                    )],
                );
                tool_summary = "Recover from read-only action without goal progress".to_string();
                observations = vec![format!(
                    "No canonical blocked action was declared. Use desktoplab.complete with a concise grounded message if the read-only evidence satisfies the user goal; otherwise select the different canonical action required by the goal. Existing executor evidence:\n{}",
                    observations.join("\n\n")
                )];
                continue;
            }
            if approved_action_completed
                && clarification_is_optional_after_mutation(&next_tool, tool)
            {
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(
                            "provider_output_recovery:optional_post_action_clarification"
                                .to_string(),
                        ),
                        SessionEvent::completed("agent loop completed"),
                    ],
                );
                return;
            }
            if approved_action_completed && repeats_completed_git_transition(tool, &next_tool) {
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(
                            "provider_output_recovery:completed_git_transition_not_repeated"
                                .to_string(),
                        ),
                        SessionEvent::completed("agent loop completed"),
                    ],
                );
                return;
            }
            if self.continuation_action_already_applied(session_id, &next_tool, &response) {
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:deduplicated_applied_action".to_string(),
                    )],
                );
                tool_summary = format!("{next_tool:?}");
                observations = vec![format!(
                    "The requested tool action was already applied in this session. Executor evidence:\n{}",
                    observations.join("\n\n")
                )];
                continue;
            }
            if self.continuation_action_already_failed(session_id, &next_tool, &response) {
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(
                        "provider_output_recovery:deduplicated_failed_action".to_string(),
                    )],
                );
                if no_progress_recovery_used {
                    self.sessions
                        .fail(session_id, "agent_no_progress_repeated_failed_action");
                    return;
                }
                no_progress_recovery_used = true;
                tool_summary =
                    "Choose a materially different action after executor failure".to_string();
                observations = vec![format!(
                    "That exact canonical action and payload already failed in this session. It will not be offered for approval again. Use current executor evidence to choose a materially different action, such as a full write or a patch grounded in the exact latest read. Existing executor evidence:\n{}",
                    observations.join("\n\n")
                )];
                continue;
            }
            if approved_action_completed && repeats_completed_filesystem_target(tool, &next_tool) {
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(
                            "provider_output_recovery:completed_target_not_rewritten".to_string(),
                        ),
                        SessionEvent::completed("agent loop completed"),
                    ],
                );
                return;
            }
            let mut decision_events = Vec::new();
            if let Some(message) = display_backend_response(&response) {
                decision_events.push(SessionEvent::backend_response_received(message));
            }
            decision_events.push(SessionEvent::tool_decision_recorded(tool_decision_message(
                "planned", &next_tool,
            )));
            self.sessions.append_events(session_id, &decision_events);
            if tool_has_immediate_observation(Some(&next_tool)) {
                if previous_immediate_tool
                    .as_ref()
                    .is_some_and(|previous| equivalent_read_only_action(previous, &next_tool))
                {
                    let unresolved_failed_validation =
                        self.session_has_unresolved_failed_validation(session_id);
                    if approved_action_completed && !unresolved_failed_validation {
                        self.sessions.append_events(
                            session_id,
                            &[
                                SessionEvent::tool_decision_recorded(
                                    "provider_output_recovery:repeated_read_only_action"
                                        .to_string(),
                                ),
                                SessionEvent::completed("agent loop completed"),
                            ],
                        );
                        return;
                    }
                    if no_progress_recovery_used
                        && (unresolved_failed_validation || final_read_only_recovery_used)
                    {
                        self.sessions
                            .fail(session_id, "agent_no_progress_repeated_read_only_action");
                        return;
                    }
                    let recovery_kind = if no_progress_recovery_used {
                        final_read_only_recovery_used = true;
                        "provider_output_recovery:final_read_only_synthesis"
                    } else {
                        no_progress_recovery_used = true;
                        "provider_output_recovery:read_only_no_progress"
                    };
                    self.sessions.append_events(
                        session_id,
                        &[SessionEvent::tool_decision_recorded(
                            recovery_kind.to_string(),
                        )],
                    );
                    tool_summary = if final_read_only_recovery_used {
                        "Final synthesis from existing read-only executor evidence".to_string()
                    } else {
                        "Recover from repeated read-only action without progress".to_string()
                    };
                    let recovery = if unresolved_failed_validation {
                        "The latest validation still fails. Inspect different repository evidence, repair the root cause, and rerun validation before completion."
                    } else if final_read_only_recovery_used {
                        "The same read-only action has already been rejected as redundant. Do not call it again. Return desktoplab.complete now when the existing evidence answers the goal; otherwise choose one materially different canonical action."
                    } else {
                        "Use desktoplab.complete with a concise grounded message if the goal is already satisfied; otherwise select a different canonical action."
                    };
                    observations = vec![format!(
                        "The repeated read-only action added no new evidence. {recovery} Existing executor evidence:\n{}",
                        observations.join("\n\n")
                    )];
                    continue;
                }
                self.sessions.append_events(
                    session_id,
                    &[SessionEvent::tool_decision_recorded(tool_decision_message(
                        "executed", &next_tool,
                    ))],
                );
                self.append_filesystem_read_observation(session_id, Some(&next_tool));
                self.append_git_observation(session_id, Some(&next_tool));
                self.append_workspace_tool_observation(session_id, Some(&next_tool));
                if !self.session_can_continue_after_observation(session_id) {
                    return;
                }
                previous_immediate_tool = Some(next_tool.clone());
                tool_summary = format!("{next_tool:?}");
                observations.push(self.latest_agent_observation(session_id));
                continue;
            }
            if self.pause_continuation_for_approval(session_id, &next_tool, &response)
                == AgentApprovalRequestOutcome::CheckpointBlocked
            {
                tool_summary =
                    "Choose a policy-compatible action after checkpoint refusal".to_string();
                observations.push(
                    "The requested terminal action was not approved because a clean Git checkpoint could not be prepared for the dirty worktree. The action did not run. Use a purpose-built read, test, diff, or other canonical tool that satisfies the goal without bypassing policy."
                        .to_string(),
                );
                continue;
            }
            return;
        }
        self.sessions
            .fail(session_id, "agent_continuation_max_steps");
    }

    #[cfg(debug_assertions)]
    fn recover_initial_checkpoint_refusal(
        &mut self,
        session_id: &str,
        backend_id: &str,
        tool: Option<&ToolIntent>,
    ) {
        self.sessions.append_events(
            session_id,
            &[SessionEvent::tool_decision_recorded(
                "provider_output_recovery:initial_checkpoint_refusal".to_string(),
            )],
        );
        let Some(tool) = tool else {
            self.sessions
                .block(session_id, "checkpoint blocked risky mutation");
            return;
        };
        let user_goal = self
            .sessions
            .get(session_id)
            .and_then(|session| session.plans().last().cloned())
            .unwrap_or_else(|| "Complete the current repository task.".to_string());
        let observation = "The requested terminal action was not approved because a clean Git checkpoint could not be prepared for the dirty worktree. The action did not run. Use a purpose-built read, test, diff, or other canonical tool that satisfies the goal without bypassing policy.";
        let Ok(response) = self.continue_agent_after_approved_tool(
            backend_id,
            &user_goal,
            &format!("{tool:?}"),
            &[observation.to_string()],
        ) else {
            self.sessions
                .block(session_id, "checkpoint blocked risky mutation");
            return;
        };
        let Some(next_tool) = structured_action_tool(&response) else {
            self.sessions
                .block(session_id, "checkpoint blocked risky mutation");
            return;
        };
        let mut events = Vec::new();
        if let Some(message) = display_backend_response(&response) {
            events.push(SessionEvent::backend_response_received(message));
        }
        events.push(SessionEvent::tool_decision_recorded(tool_decision_message(
            "planned", &next_tool,
        )));
        self.sessions.append_events(session_id, &events);
        if tool_has_immediate_observation(Some(&next_tool)) {
            self.sessions.append_events(
                session_id,
                &[SessionEvent::tool_decision_recorded(tool_decision_message(
                    "executed", &next_tool,
                ))],
            );
            self.append_filesystem_read_observation(session_id, Some(&next_tool));
            self.append_git_observation(session_id, Some(&next_tool));
            self.append_workspace_tool_observation(session_id, Some(&next_tool));
            if self.session_can_continue_after_observation(session_id) {
                self.finish_observation_continuation(
                    session_id,
                    backend_id,
                    Some(&next_tool),
                    false,
                    None,
                );
            }
            return;
        }
        if self.pause_continuation_for_approval(session_id, &next_tool, &response)
            == AgentApprovalRequestOutcome::CheckpointBlocked
        {
            self.sessions
                .block(session_id, "checkpoint blocked risky mutation");
        }
    }

    #[cfg(debug_assertions)]
    fn latest_agent_observation(&self, session_id: &str) -> String {
        self.sessions
            .get(session_id)
            .and_then(|session| session.backend_responses().last().cloned())
            .unwrap_or_else(|| "Tool completed without textual output.".to_string())
    }

    #[cfg(debug_assertions)]
    fn session_has_unresolved_failed_validation(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .and_then(|session| {
                session
                    .backend_responses()
                    .iter()
                    .rev()
                    .find(|message| message.starts_with("Test command `"))
                    .cloned()
            })
            .is_some_and(|message| !message.contains("status Exited(0)"))
    }

    #[cfg(debug_assertions)]
    fn latest_session_test_run(&self, session_id: &str) -> Option<ToolIntent> {
        self.agent_pending_actions
            .values()
            .filter(|pending| pending.session_id() == session_id)
            .find_map(|pending| match pending.tool() {
                tool @ ToolIntent::TestRun { .. } => Some(tool.clone()),
                _ => None,
            })
    }

    #[cfg(debug_assertions)]
    fn pause_for_automatic_validation_rerun(&mut self, session_id: &str) -> bool {
        let Some(rerun_tool) = self.latest_session_test_run(session_id) else {
            return false;
        };
        self.sessions.append_events(
            session_id,
            &[SessionEvent::tool_decision_recorded(
                "provider_output_recovery:automatic_validation_rerun".to_string(),
            )],
        );
        self.pause_continuation_for_approval(session_id, &rerun_tool, "{}");
        true
    }

    #[cfg(debug_assertions)]
    fn continuation_action_already_applied(
        &self,
        session_id: &str,
        tool: &ToolIntent,
        response: &str,
    ) -> bool {
        let content = pending_content_for_tool(tool, response);
        let candidate = PendingAgentAction::new(
            "continuation.candidate",
            session_id.to_string(),
            tool.clone(),
            content,
            is_filesystem_mutation(tool),
        );
        self.agent_pending_actions.values().any(|pending| {
            pending.session_id() == session_id
                && pending.state() == PendingAgentActionState::Applied
                && pending.payload_hash() == candidate.payload_hash()
        }) || self.filesystem_mutation_postcondition_is_satisfied(
            session_id,
            tool,
            candidate.content(),
        )
    }

    #[cfg(debug_assertions)]
    fn filesystem_mutation_postcondition_is_satisfied(
        &self,
        session_id: &str,
        tool: &ToolIntent,
        content: Option<&str>,
    ) -> bool {
        let Some(path) = filesystem_mutation_path(tool) else {
            return false;
        };
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            return false;
        };
        let mut executor = FilesystemToolExecutor::new(
            Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        match executor.read(path) {
            FilesystemToolOutcome::Read(current) => {
                filesystem_mutation_postcondition_is_satisfied(tool, content, &current)
            }
            _ => false,
        }
    }

    #[cfg(debug_assertions)]
    fn continuation_action_already_failed(
        &self,
        session_id: &str,
        tool: &ToolIntent,
        response: &str,
    ) -> bool {
        let content = pending_content_for_tool(tool, response);
        let candidate = PendingAgentAction::new(
            "continuation.candidate",
            session_id.to_string(),
            tool.clone(),
            content,
            is_filesystem_mutation(tool),
        );
        self.agent_pending_actions.values().any(|pending| {
            pending.session_id() == session_id
                && pending.state() == PendingAgentActionState::Failed
                && pending.payload_hash() == candidate.payload_hash()
        })
    }

    #[cfg(debug_assertions)]
    fn pause_continuation_for_approval(
        &mut self,
        session_id: &str,
        tool: &ToolIntent,
        decision_response: &str,
    ) -> AgentApprovalRequestOutcome {
        let Some(workspace) = self.execution_workspace_record(session_id) else {
            self.sessions
                .fail(session_id, "execution_workspace_unavailable");
            return AgentApprovalRequestOutcome::Skipped;
        };
        let workspace_root = PathBuf::from(workspace.root_path);
        let no_op_git_commit = git_commit_content_has_no_changes(
            tool,
            git_commit_pending_content(tool, &workspace_root).as_deref(),
        );
        let Some(session) = self.sessions.get(session_id) else {
            return AgentApprovalRequestOutcome::Skipped;
        };
        let outcome = match self.request_agent_tool_approval(
            session_id,
            Some(tool),
            &session,
            Some(decision_response),
            "continuation",
            &workspace_root,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.sessions
                    .fail(session_id, format!("state_journal_failed:{error}"));
                return AgentApprovalRequestOutcome::PersistenceFailed;
            }
        };
        match outcome {
            AgentApprovalRequestOutcome::Created | AgentApprovalRequestOutcome::Deduplicated => {
                self.sessions.block(session_id, "waiting for approval");
            }
            AgentApprovalRequestOutcome::CheckpointBlocked => {}
            AgentApprovalRequestOutcome::Malformed => self
                .sessions
                .fail(session_id, "malformed structured file action"),
            AgentApprovalRequestOutcome::Skipped if no_op_git_commit => {}
            AgentApprovalRequestOutcome::Skipped => self
                .sessions
                .fail(session_id, "unsupported continuation tool"),
            AgentApprovalRequestOutcome::PersistenceFailed => {}
        }
        outcome
    }

    #[cfg(debug_assertions)]
    fn session_can_continue_after_observation(&self, session_id: &str) -> bool {
        self.sessions.get(session_id).is_some_and(|session| {
            !matches!(
                session.state(),
                desktoplab_agent_session::SessionState::Blocked
                    | desktoplab_agent_session::SessionState::Failed
            )
        })
    }

    #[cfg(debug_assertions)]
    fn workspace_checkpoint_observation(
        &self,
        root: &Path,
        session_id: &str,
        label: &str,
    ) -> Result<String, String> {
        let checkpoint_ref = format!(
            "checkpoint.agent.{}.{}",
            session_id,
            stable_label_fragment(label)
        );
        let mut executor = GitToolExecutor::new(root, PolicyEngine::default_conservative());
        match executor.prepare_checkpoint_ref(checkpoint_ref) {
            GitToolOutcome::CheckpointReady(id) => Ok(format!("Checkpoint ready: {id}")),
            GitToolOutcome::Blocked(reason) => Err(reason.to_string()),
            _ => Err("checkpoint_failed".to_string()),
        }
    }

    #[cfg(debug_assertions)]
    fn read_pending_write_observation(
        &self,
        pending: &PendingAgentAction,
    ) -> Option<Result<(ToolIntent, String), String>> {
        if !pending.readback_after_write() {
            return None;
        }
        let (ToolIntent::FilesystemWrite { path } | ToolIntent::FilesystemPatch { path }) =
            pending.tool()
        else {
            return None;
        };
        let Some(workspace) = self.execution_workspace_record(pending.session_id()) else {
            return Some(Err("execution_workspace_unavailable".to_string()));
        };
        let mut executor = FilesystemToolExecutor::new(
            std::path::Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        match executor.read(path) {
            FilesystemToolOutcome::Read(contents) => Some(Ok((
                ToolIntent::filesystem_read(path),
                format!("Read {path}:\n{contents}"),
            ))),
            FilesystemToolOutcome::Blocked(reason) => Some(Err(reason.to_string())),
            FilesystemToolOutcome::ApprovalRequired => Some(Err(
                "filesystem read unexpectedly required approval".to_string(),
            )),
            FilesystemToolOutcome::Denied => Some(Err("filesystem read denied".to_string())),
            FilesystemToolOutcome::Written => {
                Some(Err("unexpected filesystem write observation".to_string()))
            }
            FilesystemToolOutcome::Unchanged => Some(Err(
                "unexpected unchanged filesystem observation".to_string(),
            )),
        }
    }

    #[cfg(debug_assertions)]
    fn apply_approved_agent_tool(
        &self,
        tool: Option<&ToolIntent>,
        session: &desktoplab_agent_session::AgentSession,
    ) {
        let Some(ToolIntent::FilesystemWrite { path }) = tool else {
            return;
        };
        let Some(contents) = session.backend_responses().last() else {
            return;
        };
        let Some(workspace) = self.execution_workspace_record(session.session_id()) else {
            return;
        };
        let mut executor = FilesystemToolExecutor::new(
            std::path::Path::new(&workspace.root_path),
            PolicyEngine::default_conservative(),
        );
        let _ = match executor.write(path, contents, FilesystemApproval::Approved) {
            FilesystemToolOutcome::Written | FilesystemToolOutcome::Unchanged => Ok(()),
            _ => Err(()),
        };
    }

    fn provider_egress_approval_state(
        &mut self,
        body: &str,
        workspace_id: &str,
        backend_id: &str,
        prompt: &str,
        context_paths: &[String],
        external_attachments: &[serde_json::Value],
    ) -> ProviderEgressState {
        if !is_external_backend(backend_id)
            || (context_paths.is_empty() && external_attachments.is_empty())
        {
            return ProviderEgressState::NotNeeded;
        }
        let operation_id = provider_egress_operation_id(workspace_id);
        let payload_hash = provider_egress_payload_hash(
            workspace_id,
            backend_id,
            prompt,
            context_paths,
            external_attachments,
        );
        if let Some(approval_id) = body_field(body, "approvalId") {
            match self.consume_body_approved_record(
                body,
                "session.pending",
                "provider.egress",
                &operation_id,
                Some(&payload_hash),
            ) {
                Ok(true) => return ProviderEgressState::Allowed,
                Err(error) => {
                    return ProviderEgressState::Blocked(ApiRouteResponse::state_journal_failed(
                        error,
                    ));
                }
                Ok(false) => {}
            }
            if self.approvals.get(&approval_id).is_some_and(|record| {
                record.state() == ApprovalState::Denied
                    && record.action() == "provider.egress"
                    && record.operation_id() == operation_id
                    && record.payload_hash() == Some(payload_hash.as_str())
            }) {
                return ProviderEgressState::Blocked(provider_egress_blocked_response(
                    "provider_egress_denied",
                    workspace_id,
                    backend_id,
                    None,
                ));
            }
        }
        let approval = self.approvals.request_operation_with_payload_hash(
            "session.pending",
            "provider.egress",
            operation_id,
            Some(payload_hash),
        );
        ProviderEgressState::Blocked(provider_egress_blocked_response(
            "provider_egress_approval_required",
            workspace_id,
            backend_id,
            Some(approval_json(&approval)),
        ))
    }

    #[cfg(debug_assertions)]
    fn agent_request_and_adapter(
        &mut self,
        step: FirstPromptStep,
        backend_id: &str,
        compiled_prompt: &str,
    ) -> (
        desktoplab_agent_engine::AgentRunRequest,
        LlmExecutionAdapter,
        InitialBackendRecoveryState,
    ) {
        let request = step.request();
        match &mut self.agent_backend_execution {
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::DeterministicForTest(output) => (
                request.with_backend_response(output.clone()),
                LlmExecutionAdapter::local(backend_id),
                InitialBackendRecoveryState::NotNeeded,
            ),
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::DeterministicSequenceForTest(outputs) => (
                request.with_backend_response(if outputs.is_empty() {
                    String::new()
                } else {
                    outputs.remove(0)
                }),
                LlmExecutionAdapter::local(backend_id),
                InitialBackendRecoveryState::NotNeeded,
            ),
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs) => (
                request.with_backend_response(if outputs.is_empty() {
                    String::new()
                } else {
                    outputs.remove(0)
                }),
                LlmExecutionAdapter::local(backend_id),
                InitialBackendRecoveryState::NotNeeded,
            ),
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::FailForTest => (
                request,
                LlmExecutionAdapter::local(backend_id).with_local_inference_failure(),
                InitialBackendRecoveryState::NotNeeded,
            ),
            AgentBackendExecutionMode::Execute => {
                let Ok(tool_ids) = self.agent_tool_ids() else {
                    return (
                        request,
                        adapter_for_backend(backend_id).with_external_backend_unavailable(),
                        InitialBackendRecoveryState::NotNeeded,
                    );
                };
                let initial =
                    self.run_selected_backend_with_transient_retry(backend_id, compiled_prompt);
                let recovered = initial.and_then(|output| {
                    recover_initial_backend_output(
                        output,
                        compiled_prompt,
                        provider_output_requires_initial_retry,
                        |user_goal| {
                            super::agent_continuation::initial_tool_recovery_prompt(
                                user_goal, &tool_ids,
                            )
                        },
                        |retry_prompt| {
                            self.run_selected_backend_with_transient_retry(backend_id, retry_prompt)
                        },
                    )
                });
                match recovered {
                    Ok(recovered) => (
                        request.with_backend_response(recovered.output),
                        adapter_for_backend(backend_id),
                        recovered.recovery,
                    ),
                    Err(()) => (
                        request,
                        adapter_for_backend(backend_id).with_external_backend_unavailable(),
                        InitialBackendRecoveryState::NotNeeded,
                    ),
                }
            }
        }
    }

    pub(crate) fn selected_execution_backend_id(&self) -> String {
        if self.selected_route_id == "route.external.codex" {
            "backend.codex".to_string()
        } else if self.selected_route_id == "route.high-end-local"
            && self.high_end_runtime.is_some()
        {
            "backend.high-end-local".to_string()
        } else if self.selected_local_runtime_id().as_deref() == Some("runtime.mlx-lm") {
            "backend.mlx-lm".to_string()
        } else if self.selected_local_runtime_id().as_deref() == Some("runtime.lm-studio") {
            "backend.lm-studio".to_string()
        } else {
            "backend.ollama".to_string()
        }
    }

    pub(super) fn run_selected_backend(
        &self,
        backend_id: &str,
        prompt: &str,
    ) -> Result<String, ()> {
        if backend_id == "backend.codex" {
            return self.run_codex_responder(prompt);
        }
        if backend_id == "backend.mlx-lm" {
            return self.run_local_mlx_lm(prompt);
        }
        if backend_id == "backend.lm-studio" {
            return self.run_local_lm_studio(prompt);
        }
        if backend_id == "backend.high-end-local" {
            return self.run_high_end_local(prompt);
        }
        self.run_local_ollama(prompt)
    }

    pub(super) fn run_selected_backend_messages(
        &mut self,
        backend_id: &str,
        messages: Vec<desktoplab_backends::BackendMessage>,
    ) -> Result<String, super::agent_model_execution::AgentModelExecutionError> {
        #[cfg(debug_assertions)]
        if let AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs) =
            &mut self.agent_backend_execution
        {
            return if outputs.is_empty() {
                Err(
                    super::agent_model_execution::AgentModelExecutionError::runtime(
                        "native_iterative_test_backend_exhausted",
                    ),
                )
            } else {
                Ok(outputs.remove(0))
            };
        }
        match backend_id {
            "backend.ollama" => self
                .run_local_ollama_messages(messages)
                .map_err(super::agent_model_execution::AgentModelExecutionError::from_backend),
            "backend.lm-studio" => self
                .run_local_lm_studio_messages(messages)
                .map_err(super::agent_model_execution::AgentModelExecutionError::from_backend),
            "backend.high-end-local" => self
                .run_high_end_local_messages(messages)
                .map_err(super::agent_model_execution::AgentModelExecutionError::from_backend),
            "backend.codex" => self
                .run_codex_responder_messages(messages)
                .map_err(super::agent_model_execution::AgentModelExecutionError::from_backend),
            "backend.mlx-lm" => {
                let prompt = constrained_backend_prompt(&messages)
                    .map_err(super::agent_model_execution::AgentModelExecutionError::runtime)?;
                self.run_selected_backend(backend_id, &prompt).map_err(|_| {
                    super::agent_model_execution::AgentModelExecutionError::runtime(
                        "constrained_backend_execution_failed",
                    )
                })
            }
            _ => Err(
                super::agent_model_execution::AgentModelExecutionError::runtime(
                    "backend_native_tool_history_unsupported",
                ),
            ),
        }
    }

    fn run_codex_responder_messages(
        &self,
        messages: Vec<desktoplab_backends::BackendMessage>,
    ) -> Result<String, String> {
        let (responder_url, payload) = self.codex_agent_execution_request(messages)?;
        desktoplab_backends::execute_openai_codex_responder_command(&responder_url, &payload)
            .map(|output| output.body().to_string())
    }

    #[cfg(debug_assertions)]
    fn run_selected_backend_with_transient_retry(
        &self,
        backend_id: &str,
        prompt: &str,
    ) -> Result<String, ()> {
        let attempts = if backend_id == "backend.codex" {
            1
        } else {
            LOCAL_BACKEND_TRANSPORT_ATTEMPTS
        };
        retry_backend_transport(attempts, Duration::from_millis(250), || {
            self.run_selected_backend(backend_id, prompt)
        })
    }

    fn run_codex_responder(&self, prompt: &str) -> Result<String, ()> {
        let account = self.provider_accounts.get("provider.openai").ok_or(())?;
        if !account.is_codex_bridge_ready() {
            return Err(());
        }
        let vault_ref = account.vault_ref().ok_or(())?;
        let responder_url = account.bridge_responder_url().ok_or(())?;
        if !self.codex_credential_available(vault_ref) {
            return Err(());
        }
        let payload = desktoplab_backends::OpenAiCodexResponderCommandPayload::new(
            prompt,
            vault_ref,
            account.vault_kind().unwrap_or("native_vault"),
        )
        .map_err(|_| ())?;
        let output =
            desktoplab_backends::execute_openai_codex_responder_command(responder_url, &payload)
                .map_err(|_| ())?;
        Ok(output.body().to_string())
    }

    pub(crate) fn codex_credential_available(&self, vault_ref: &str) -> bool {
        let Ok(secret_ref) = desktoplab_vault::SecretRef::from_uri(vault_ref) else {
            return false;
        };
        if let Some(vault) = &self.openai_codex_native_vault_for_test {
            return desktoplab_vault::Vault::get(vault, &secret_ref).is_ok();
        }
        desktoplab_vault::get_current_native_secret(&secret_ref).is_ok()
    }

    fn run_local_ollama(&self, prompt: &str) -> Result<String, ()> {
        self.run_local_ollama_messages(vec![desktoplab_backends::BackendMessage::user(prompt)])
            .map_err(|_| ())
    }

    fn run_local_ollama_messages(
        &self,
        messages: Vec<desktoplab_backends::BackendMessage>,
    ) -> Result<String, String> {
        let model_id = self
            .selected_local_model_id()
            .map_err(|_| "local_model_unavailable".to_string())?;
        let pull_ref = crate::model_routes::model_pull_ref(&model_id)
            .ok_or_else(|| "local_model_pull_reference_missing".to_string())?;
        let endpoint = "http://127.0.0.1:11434";
        let mut model_capabilities = self
            .ollama_model_capabilities
            .resolve(endpoint, &pull_ref)
            .map_err(|error| format!("ollama_capability_resolution_failed:{error}"))?;
        let certification = self
            .readiness
            .model_capabilities()
            .filter(|current| current.fingerprint() == model_capabilities.fingerprint())
            .and_then(|current| current.tool_protocol_certification())
            .filter(|certification| {
                certification.is_certified_for(model_capabilities.fingerprint())
            })
            .cloned()
            .ok_or_else(|| "model_tool_protocol_uncertified".to_string())?;
        model_capabilities = model_capabilities.with_tool_protocol_certification(certification);
        let context_window_tokens = crate::model_routes::agent_context_window_tokens(
            &model_id,
            self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
        )
        .ok_or_else(|| "local_model_context_window_unavailable".to_string())?;
        let request_timeout_seconds = crate::model_routes::agent_request_timeout_seconds(
            &model_id,
            self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
        )
        .ok_or_else(|| "local_model_request_timeout_unavailable".to_string())?;
        let prompt = desktoplab_backends::BackendPrompt::new(pull_ref.clone(), "")
            .with_messages(messages)
            .with_tools(self.backend_tool_schemas()?)
            .with_context_window_tokens(context_window_tokens)
            .with_request_timeout_seconds(request_timeout_seconds);
        let backend = desktoplab_backends::OllamaExecutionBackend::new(
            desktoplab_backends::BackendModelInventory::available(&[pull_ref.as_str()]),
        )
        .with_model_capabilities([model_capabilities]);
        backend.execute_chat(endpoint, &prompt)
    }

    fn run_local_mlx_lm(&self, prompt: &str) -> Result<String, ()> {
        let model_id = self.selected_local_model_id()?;
        let pull_ref = crate::model_routes::model_pull_ref(&model_id).ok_or(())?;
        let output = <SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ProcessCommand::new("mlx_lm.generate")
                .arg("--model")
                .arg(pull_ref)
                .arg("--prompt")
                .arg(prompt),
        );
        if !output.succeeded() {
            return Err(());
        }
        let response = output.stdout().trim();
        if response.is_empty() {
            return Err(());
        }
        Ok(response.to_string())
    }

    fn run_local_lm_studio(&self, prompt: &str) -> Result<String, ()> {
        self.run_local_lm_studio_messages(vec![desktoplab_backends::BackendMessage::user(prompt)])
            .map_err(|_| ())
    }

    fn run_local_lm_studio_messages(
        &self,
        messages: Vec<desktoplab_backends::BackendMessage>,
    ) -> Result<String, String> {
        let model_id = self
            .selected_local_model_id()
            .map_err(|_| "local_model_unavailable".to_string())?;
        let pull_ref = crate::model_routes::model_pull_ref(&model_id).unwrap_or(model_id);
        let prompt = desktoplab_backends::BackendPrompt::new(pull_ref.clone(), "")
            .with_messages(messages)
            .with_tools(self.backend_tool_schemas()?);
        let backend = desktoplab_backends::LmStudioExecutionBackend::new(
            desktoplab_backends::LocalEndpoint::available("http://127.0.0.1:1234"),
            desktoplab_backends::BackendModelInventory::available(&[pull_ref.as_str()]),
        );
        backend.execute_chat(&prompt)
    }

    fn run_high_end_local(&self, prompt: &str) -> Result<String, ()> {
        self.run_high_end_local_messages(vec![desktoplab_backends::BackendMessage::user(prompt)])
            .map_err(|_| ())
    }

    fn run_high_end_local_messages(
        &self,
        messages: Vec<desktoplab_backends::BackendMessage>,
    ) -> Result<String, String> {
        let runtime = self
            .high_end_runtime
            .as_ref()
            .ok_or_else(|| "high_end_runtime_unavailable".to_string())?;
        if runtime.evidence().state() != desktoplab_runtime::HighEndRuntimeHealthState::ModelReady {
            return Err("high_end_model_not_ready".to_string());
        }
        let model_id = runtime.endpoint().model_id();
        let prompt = desktoplab_backends::BackendPrompt::new(model_id, "")
            .with_messages(messages)
            .with_tools(self.backend_tool_schemas()?);
        desktoplab_backends::OpenAiCompatibleLocalExecutionBackend::new(
            "backend.high-end-local",
            desktoplab_backends::LocalEndpoint::available(runtime.endpoint().base_url()),
            desktoplab_backends::BackendModelInventory::available(&[model_id]),
        )
        .execute_chat(&prompt)
    }

    pub(super) fn selected_local_model_id(&self) -> Result<String, ()> {
        crate::execution_routes::local_model_id_from_route_id(&self.selected_route_id)
            .or_else(|| self.readiness.model_id().map(ToString::to_string))
            .ok_or(())
    }

    pub(super) fn selected_local_runtime_id(&self) -> Option<String> {
        self.selected_local_model_id()
            .ok()
            .and_then(|model_id| crate::model_routes::model_runtime_id(&model_id))
            .or_else(|| self.readiness.runtime_id().map(ToString::to_string))
    }

    pub(crate) fn sessions_list(&self, path: &str) -> ApiRouteResponse {
        let workspace_id = query_value(path, "workspace_id")
            .or_else(|| self.workspace_id())
            .unwrap_or_default();
        let sessions = self
            .sessions
            .list_by_workspace(&workspace_id)
            .iter()
            .filter(|session| !self.archived_session_ids.contains(session.session_id()))
            .map(|session| {
                self.session_payload_with_pending_approvals(Some(session), &workspace_id)
            })
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({"sessions":sessions}))
    }

    pub(crate) fn session_control(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let session_id = segment(path, 2);
        let Some(session) = self.sessions.get(&session_id) else {
            return ApiRouteResponse::not_found();
        };
        let Some(workspace_id) = self.sessions.workspace_id_for(&session_id) else {
            self.sessions
                .fail(&session_id, "execution_workspace_unavailable");
            return ApiRouteResponse::bad_request(json!({
                "code":"EXECUTION_WORKSPACE_UNAVAILABLE",
                "message":"The session no longer has a valid workspace binding."
            }));
        };
        let backend_id = session.execution_backend_id().to_string();
        match body_field_or(body, "action", "").as_str() {
            "cancel" => {
                if let Some(token) = self.agent_cancellation_tokens.get(&session_id) {
                    token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                let _ = self
                    .agent_process_registry
                    .kill_session(&workspace_id, &session_id);
                if let Some(state) = self.agent_iterative_states.get_mut(&session_id) {
                    state.cancel("user cancelled");
                }
                self.approvals
                    .invalidate_unconsumed_for_session(&session_id);
                for pending in self
                    .agent_pending_actions
                    .values_mut()
                    .filter(|pending| pending.session_id() == session_id)
                {
                    if matches!(
                        pending.state(),
                        PendingAgentActionState::Pending | PendingAgentActionState::Applying
                    ) {
                        pending.mark_failed();
                    }
                }
                self.agent_streaming_sessions.remove(&session_id);
                self.sessions
                    .request_cancel(&session_id, current_time_millis(), 2_000);
                self.sessions.acknowledge_cancel(&session_id);
                self.sessions.cancel(&session_id, "user cancelled");
                if let Err(error) = self.persist_agent_approval_journal() {
                    return ApiRouteResponse::state_journal_failed(error);
                }
                self.events.publish_agent_event(
                    "agent.stream.cancelled",
                    &workspace_id,
                    &session_id,
                    &backend_id,
                    "Streaming response cancelled",
                );
                ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                    self.sessions.get(&session_id).as_ref(),
                    &workspace_id,
                ))
            }
            "pause" => {
                if session.state() != desktoplab_agent_session::SessionState::Running {
                    return invalid_session_control_state("pause", session.state());
                }
                if let Some(token) = self.agent_cancellation_tokens.get(&session_id) {
                    token.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.sessions.pause(&session_id, "user paused");
                ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                    self.sessions.get(&session_id).as_ref(),
                    &workspace_id,
                ))
            }
            "resume" => {
                let resumable = session.state() == desktoplab_agent_session::SessionState::Paused
                    || (session.state() == desktoplab_agent_session::SessionState::Blocked
                        && session.blocked_reason() == Some("long_running_job_interrupted"));
                if !resumable {
                    return invalid_session_control_state("resume", session.state());
                }
                self.agent_cancellation_tokens.insert(
                    session_id.clone(),
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                );
                self.sessions.resume(&session_id);
                if let Some(turn) = self.sessions.claim_next_turn(&session_id) {
                    let response = self.continue_session(
                        &format!("/v1/sessions/{session_id}/messages"),
                        &json!({"workspaceId":workspace_id,"executionBackendId":backend_id,"prompt":turn.prompt(),"queuedReplay":true}).to_string(),
                        AgentContinuationMode::Immediate,
                    );
                    if response.status() == "200 OK" {
                        self.sessions.complete_turn(&session_id, turn.turn_id());
                        return ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                            self.sessions.get(&session_id).as_ref(),
                            &workspace_id,
                        ));
                    }
                    return response;
                }
                ApiRouteResponse::ok(self.session_payload_with_pending_approvals(
                    self.sessions.get(&session_id).as_ref(),
                    &workspace_id,
                ))
            }
            _ => ApiRouteResponse::bad_request(json!({
                "code":"UNKNOWN_SESSION_CONTROL",
                "message":"Session control action must be pause, resume or cancel."
            })),
        }
    }

    pub(crate) fn jobs_list(&self) -> ApiRouteResponse {
        let jobs = self
            .jobs
            .list_jobs()
            .iter()
            .map(|job| {
                json!({
                    "jobId":job.id().as_str(),
                    "kind":job.kind(),
                    "state":job_state_value(job.state()),
                    "progressPercent":if job.state() == JobState::Succeeded { 100 } else { 0 },
                    "retryClass":"unknown",
                    "updatedAt":"2026-06-26T00:00:00Z"
                })
            })
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({"source":"service_backed","jobs":jobs}))
    }

    pub(crate) fn retry_job(&mut self, path: &str) -> ApiRouteResponse {
        let job_id = super::helpers::segment(path, 2);
        let job_id = JobId::new(job_id);
        match self.jobs.retry(&job_id) {
            Ok(()) => {
                self.persist_runtime_jobs();
                self.persist_model_jobs();
                ApiRouteResponse::ok(json!({
                    "source":"service_backed",
                    "accepted":true,
                    "jobId":job_id.as_str(),
                    "state":"queued"
                }))
            }
            Err(error) => ApiRouteResponse::bad_request(json!({
                "code":"JOB_RETRY_FAILED",
                "message":error
            })),
        }
    }

    pub(crate) fn events_replay(&self, path: &str) -> ApiRouteResponse {
        let after_sequence = after_sequence_cursor(path);
        let mut request = EventReplayRequest::new().after_sequence(after_sequence);
        if let Some(stream_id) = query_value(path, "stream_id") {
            request = request.expected_stream_id(stream_id);
        }
        if let Some(limit) = query_value(path, "limit").and_then(|value| value.parse().ok()) {
            request = request.max_events(limit);
        }
        let replay = self.events.replay(request);
        let frames = replay
            .frames()
            .iter()
            .map(event_frame_json)
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "streamId":replay.stream_id(),
            "oldestSequence":replay.oldest_sequence(),
            "latestSequence":replay.latest_sequence(),
            "nextSequence":replay.next_sequence(),
            "hasMore":replay.has_more(),
            "gapDetected":replay.gap_detected(),
            "resetRequired":replay.reset_required(),
            "frames":frames
        }))
    }

    pub(crate) fn create_bound_agent_session(
        &mut self,
        workspace_id: &str,
        backend_id: &str,
    ) -> desktoplab_agent_session::AgentSession {
        let binding =
            super::agent_execution_binding::AgentExecutionBinding::capture(self, backend_id);
        let session = self.sessions.create_session(workspace_id, backend_id);
        self.agent_execution_bindings
            .insert(session.session_id().to_string(), binding);
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
        }
        session
    }

    fn inherit_agent_execution_binding(
        &mut self,
        session_id: &str,
        parent_session_id: Option<&str>,
    ) {
        let Some(binding) = parent_session_id
            .and_then(|parent_id| self.agent_execution_bindings.get(parent_id))
            .cloned()
        else {
            return;
        };
        self.agent_execution_bindings
            .insert(session_id.to_string(), binding);
        if let Err(error) = self.persist_agent_approval_journal() {
            self.record_state_journal_result(Err(error));
        }
    }

    pub(crate) fn recover_interrupted_agent_jobs(&mut self) {
        self.sessions.interrupt_running_jobs(
            "long_running_job_interrupted",
            "Recover by reviewing partial evidence and starting a new prompt.",
        );
        let applying = self
            .agent_pending_actions
            .values()
            .filter(|pending| pending.state() == PendingAgentActionState::Applying)
            .cloned()
            .collect::<Vec<_>>();
        for pending in applying {
            if pending.iterative_call().is_some()
                && self.interrupted_action_postcondition_is_satisfied(&pending)
            {
                self.reconcile_interrupted_action(pending);
            } else {
                if let Some(action) = self.agent_pending_actions.get_mut(pending.approval_id()) {
                    action.mark_interrupted();
                }
                self.sessions.fail(
                    pending.session_id(),
                    "interrupted_action_requires_workspace_review",
                );
            }
        }
        if !self.agent_pending_actions.is_empty() {
            let result = self.persist_agent_approval_journal();
            self.record_state_journal_result(result);
        }
    }

    fn reconcile_interrupted_action(&mut self, pending: PendingAgentAction) {
        let session_id = pending.session_id().to_string();
        let Some(call) = self.native_pending_call(&pending) else {
            if let Some(action) = self.agent_pending_actions.get_mut(pending.approval_id()) {
                action.mark_interrupted();
            }
            self.sessions
                .fail(&session_id, "interrupted_action_identity_mismatch");
            return;
        };
        let Some(session) = self.sessions.get(&session_id) else {
            if let Some(action) = self.agent_pending_actions.get_mut(pending.approval_id()) {
                action.mark_interrupted();
            }
            return;
        };
        let backend_id = session.execution_backend_id().to_string();
        let Some(workspace_id) = self.sessions.workspace_id_for(&session_id) else {
            if let Some(action) = self.agent_pending_actions.get_mut(pending.approval_id()) {
                action.mark_interrupted();
            }
            self.sessions
                .fail(&session_id, "execution_workspace_unavailable");
            return;
        };
        self.sessions.start_job(
            &session_id,
            format!("agent-job.{session_id}"),
            current_timestamp(),
            true,
        );
        let _ = self.defer_native_iterative_after_execution(
            &workspace_id,
            &backend_id,
            &pending,
            PendingExecutionOutcome::NativeObservation(ToolObservation::success(
                &call,
                json!({"changed":true}),
            )),
        );
    }

    fn interrupted_action_postcondition_is_satisfied(&self, pending: &PendingAgentAction) -> bool {
        let Some(workspace) = self.execution_workspace_record(pending.session_id()) else {
            return false;
        };
        let Ok(root) = WorkspaceRoot::open(Path::new(&workspace.root_path)) else {
            return false;
        };
        #[cfg(debug_assertions)]
        if pending.iterative_call().is_none()
            && let Some(files) = pending.content().and_then(pending_multi_file_patch_payload)
        {
            return files.iter().all(|file| {
                root.read_text(file.path()).is_ok_and(|current| {
                    patch_postcondition_is_satisfied(file.expected(), file.replacement(), &current)
                })
            });
        }
        match pending.tool() {
            tool
            @ (ToolIntent::FilesystemWrite { path } | ToolIntent::FilesystemPatch { path }) => {
                root.read_text(path).is_ok_and(|current| {
                    filesystem_mutation_postcondition_is_satisfied(
                        tool,
                        pending.content(),
                        &current,
                    )
                })
            }
            ToolIntent::FilesystemCreateDirectory { path } => {
                matches!(root.path_state(path), Ok(WorkspacePathState::Directory))
            }
            ToolIntent::FilesystemMove {
                source,
                destination,
            } => {
                matches!(root.path_state(source), Ok(WorkspacePathState::Missing))
                    && matches!(
                        root.path_state(destination),
                        Ok(WorkspacePathState::File | WorkspacePathState::Directory)
                    )
            }
            ToolIntent::FilesystemDelete { path, .. } => {
                matches!(root.path_state(path), Ok(WorkspacePathState::Missing))
            }
            _ => false,
        }
    }

    pub(crate) fn workspace_id(&self) -> Option<String> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.workspace_id.clone())
    }

    pub(crate) fn workspace_record(&self) -> Option<WorkspaceRecord> {
        self.workspace.clone()
    }

    pub(crate) fn workspace_record_for_id(&self, workspace_id: &str) -> Option<WorkspaceRecord> {
        self.workspaces.get(workspace_id).cloned().or_else(|| {
            self.workspace
                .as_ref()
                .filter(|workspace| workspace.workspace_id == workspace_id)
                .cloned()
        })
    }

    fn workspace_root_missing(&self, workspace_id: &str) -> bool {
        self.workspace_record_for_id(workspace_id)
            .is_some_and(|workspace| !std::path::Path::new(&workspace.root_path).is_dir())
    }
}

pub(super) fn constrained_backend_prompt(
    messages: &[desktoplab_backends::BackendMessage],
) -> Result<String, String> {
    let transcript = messages
        .iter()
        .map(|message| match message {
            desktoplab_backends::BackendMessage::User(content) => {
                json!({"role":"user","content":content})
            }
            desktoplab_backends::BackendMessage::Assistant(content) => {
                json!({"role":"assistant","content":content})
            }
            desktoplab_backends::BackendMessage::AssistantToolCall {
                call_id,
                name,
                arguments,
            } => json!({
                "role":"assistant",
                "toolCall":{"id":call_id,"name":name,"arguments":arguments}
            }),
            desktoplab_backends::BackendMessage::ToolResult {
                call_id,
                name,
                output,
            } => json!({
                "role":"tool",
                "toolResult":{"callId":call_id,"name":name,"output":output}
            }),
        })
        .collect::<Vec<_>>();
    let encoded = serde_json::to_string(&transcript)
        .map_err(|_| "constrained_backend_transcript_failed".to_string())?;
    Ok(format!(
        "Continue the DesktopLab agent conversation represented by this JSON transcript. Treat toolResult records as authoritative executor evidence and return exactly one canonical DesktopLab JSON tool call for the next turn.\n{encoded}"
    ))
}

fn after_sequence_cursor(path: &str) -> u64 {
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return 0;
    };
    query
        .split('&')
        .find_map(|part| {
            let (key, value) = part.split_once('=')?;
            (key == "after_sequence").then(|| value.parse::<u64>().ok())?
        })
        .unwrap_or(0)
}

fn context_paths(body: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("contextPaths").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|paths| {
            paths
                .iter()
                .filter_map(|path| path.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_external_backend(backend_id: &str) -> bool {
    backend_id == "backend.codex"
}

enum ProviderEgressState {
    Allowed,
    NotNeeded,
    Blocked(ApiRouteResponse),
}

fn provider_egress_operation_id(workspace_id: &str) -> String {
    format!("provider.openai:route.external.codex:{workspace_id}")
}

fn provider_egress_payload_hash(
    workspace_id: &str,
    backend_id: &str,
    prompt: &str,
    context_paths: &[String],
    external_attachments: &[serde_json::Value],
) -> String {
    super::payload_hash::stable_payload_hash(&json!({
        "providerId":"provider.openai",
        "routeId":"route.external.codex",
        "backendId":backend_id,
        "workspaceId":workspace_id,
        "initialPrompt":prompt,
        "contextPaths":context_paths,
        "externalAttachments":external_attachment_metadata(external_attachments)
    }))
}

fn provider_egress_blocked_response(
    reason: &str,
    workspace_id: &str,
    backend_id: &str,
    approval: Option<serde_json::Value>,
) -> ApiRouteResponse {
    ApiRouteResponse::ok(json!({
        "accepted":false,
        "sessionId":"session.blocked",
        "workspaceId":workspace_id,
        "executionBackendId":backend_id,
        "owner":"desktoplab",
        "state":"blocked",
        "blockedReason":reason,
        "approval":approval,
        "summary":"Approve provider egress before sending repository context outside DesktopLab.",
        "timeline":[{
            "sequence":1,
            "kind":"blocked",
            "message":reason,
            "createdAt":current_timestamp()
        }]
    }))
}

fn workspace_root_missing_session_response(
    workspace_id: &str,
    backend_id: &str,
) -> ApiRouteResponse {
    ApiRouteResponse::ok(json!({
        "accepted":false,
        "sessionId":"session.blocked",
        "workspaceId":workspace_id,
        "executionBackendId":backend_id,
        "owner":"desktoplab",
        "state":"blocked",
        "plan":"Workspace root is missing.",
        "summary":"This thread remains readable, but DesktopLab cannot run new input until the repository is relinked.",
        "blockedReason":"workspace_root_missing",
        "nextAction":"relink_workspace",
        "timeline":[{
            "sequence":1,
            "kind":"blocked",
            "message":"workspace_root_missing",
            "createdAt":current_timestamp()
        }]
    }))
}

fn workspace_not_selected_session_response(
    workspace_id: &str,
    backend_id: &str,
) -> ApiRouteResponse {
    ApiRouteResponse::ok(json!({
        "accepted":false,
        "sessionId":"session.blocked",
        "workspaceId":workspace_id,
        "executionBackendId":backend_id,
        "owner":"desktoplab",
        "state":"blocked",
        "plan":"Open a repository before starting the agent.",
        "summary":"DesktopLab needs a real local Git repository before it can run agent work.",
        "blockedReason":"workspace_not_selected",
        "nextAction":"open_workspace",
        "timeline":[{
            "sequence":1,
            "kind":"blocked",
            "message":"workspace_not_selected",
            "createdAt":current_timestamp()
        }]
    }))
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn agent_backend_prompt(
    prompt: &str,
    planned_tool: Option<&ToolIntent>,
    expects_backend_file_action: bool,
) -> String {
    let Some(tool) = planned_tool else {
        let _ = expects_backend_file_action;
        return format!(
            "You are the DesktopLab coding agent for this repository. Use the provided native DesktopLab tools whenever repository inspection, file mutation, terminal execution, validation, Git evidence, or clarification is required. Return exactly one tool call per model turn and wait for its executor observation before choosing the next action. Treat the host execution profile in repository context as authoritative for command and path syntax. Never claim a tool action happened without its returned observation. Before completion, compare executor results with every part of the user request; placeholders, outlines, or abbreviated artifacts do not satisfy substantive content requests. Continue until the request is complete or genuinely blocked.\n\n{prompt}"
        );
    };
    let contract = match tool {
        ToolIntent::FilesystemList { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can list workspace files through its local workspace index.\n- Do not invent files; use the tool evidence.".to_string()
        }
        ToolIntent::FilesystemWrite { path } => format!(
            "DesktopLab tool contract:\n- DesktopLab can create or edit `{path}` after user approval.\n- Do not say you cannot create files.\n- Use desktoplab.write_file for a new file or intentional full replacement.\n- For a localized edit to an existing file, read current content and use desktoplab.patch_file with byte-exact expected text.\n- Provide the complete content requested by the user; do not substitute a placeholder, outline, or abbreviated artifact."
        ),
        ToolIntent::FilesystemPatch { path } => format!(
            "DesktopLab tool contract:\n- DesktopLab can patch `{path}` after user approval.\n- Read the current file first and call desktoplab.patch_file with byte-exact expected text.\n- Preserve unrelated content and verify the complete user request before finishing."
        ),
        ToolIntent::FilesystemCreateDirectory { path } => format!(
            "DesktopLab tool contract:\n- DesktopLab can create `{path}` after user approval.\n- Use desktoplab.create_directory and rely on executor evidence."
        ),
        ToolIntent::FilesystemMove {
            source,
            destination,
        } => format!(
            "DesktopLab tool contract:\n- DesktopLab can move `{source}` to `{destination}` after user approval.\n- Existing destinations are never overwritten implicitly."
        ),
        ToolIntent::FilesystemDelete { path, .. } => format!(
            "DesktopLab tool contract:\n- DesktopLab can delete `{path}` after user approval.\n- Recursive deletion must be explicit and completion requires executor evidence."
        ),
        ToolIntent::FilesystemRead { path } => format!(
            "DesktopLab tool contract:\n- DesktopLab can read `{path}` through its local filesystem tool when policy allows it.\n- Do not claim you cannot access local files if DesktopLab provides the file content as tool context."
        ),
        ToolIntent::SearchText { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can search workspace text through its local workspace index.\n- Use search evidence before answering location questions.".to_string()
        }
        ToolIntent::Terminal { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can run approved terminal commands through its agent terminal tool.\n- Do not claim terminal execution is impossible; request the command through the tool plan.".to_string()
        }
        ToolIntent::ProcessStart { .. }
        | ToolIntent::ProcessPoll { .. }
        | ToolIntent::ProcessStdin { .. }
        | ToolIntent::ProcessKill { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can own long-running processes for this session.\n- Start requires approval; poll, stdin and kill require the returned process ID and matching session ownership.\n- Report only executor-observed process state and output.".to_string()
        }
        ToolIntent::TestRun { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can run approved validation commands through its test runner tool.\n- Do not claim tests passed until the backend executor reports exit status and output.".to_string()
        }
        ToolIntent::GitCommit { .. } | ToolIntent::GitPush { .. } => {
            "DesktopLab tool contract:\n- DesktopLab owns git approvals and evidence.\n- Do not claim git operations are complete until the backend executor reports success.".to_string()
        }
        ToolIntent::GitStatus | ToolIntent::GitDiff { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can inspect git status and diff as read-only evidence.\n- Do not propose a commit message until git diff evidence has been reviewed.".to_string()
        }
        ToolIntent::CreateCheckpoint { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can create local git checkpoints as workspace evidence.\n- Do not claim checkpoint creation until the backend reports success.".to_string()
        }
        ToolIntent::McpInvoke { tool_id, .. } => format!(
            "DesktopLab tool contract:\n- DesktopLab can invoke the connected MCP tool `{tool_id}` through its policy-gated runtime.\n- Treat only its returned observation as evidence and never claim invocation before approval or execution."
        ),
        ToolIntent::Clarify { .. } => {
            "DesktopLab tool contract:\n- DesktopLab can block and ask the user a clarification question when required information is missing.".to_string()
        }
        ToolIntent::RuntimeInstall { .. } => {
            "DesktopLab tool contract:\n- DesktopLab owns runtime install state and evidence.".to_string()
        }
    };
    format!("{contract}\n\nUser prompt:\n{prompt}")
}

#[cfg(debug_assertions)]
fn structured_action_tool_from_session(
    session: &desktoplab_agent_session::AgentSession,
    workspace_id: &str,
    workspace_root: &Path,
) -> Option<ToolIntent> {
    session
        .backend_responses()
        .iter()
        .rev()
        .find_map(|response| structured_action_tool(response))
        .map(|tool| scope_provider_tool_to_workspace(tool, workspace_id, workspace_root))
}

#[cfg(debug_assertions)]
fn scope_provider_tool_to_workspace(
    tool: ToolIntent,
    workspace_id: &str,
    workspace_root: &Path,
) -> ToolIntent {
    match tool {
        ToolIntent::Terminal {
            working_directory,
            command,
            risk_class,
            ..
        } => ToolIntent::terminal_workspace(
            workspace_id,
            normalize_provider_working_directory(&working_directory, workspace_root),
            command,
            risk_class,
        ),
        other => other,
    }
}

#[cfg(debug_assertions)]
fn normalize_provider_working_directory(value: &str, workspace_root: &Path) -> String {
    let value = value.trim();
    if matches!(value, "" | "." | "./" | "/" | "\\") {
        return String::new();
    }
    let path = Path::new(value);
    if path.is_absolute()
        && let Ok(relative) = path.strip_prefix(workspace_root)
    {
        return relative.to_string_lossy().into_owned();
    }
    value.to_string()
}

#[cfg(debug_assertions)]
fn append_provider_output_recovery_event(
    raw_events: &[SessionEvent],
    events: &mut Vec<SessionEvent>,
) {
    let Some(evidence) = raw_events.iter().rev().find_map(|event| match event {
        SessionEvent::BackendResponseReceived { message } => {
            provider_output_recovery_evidence(message)
        }
        _ => None,
    }) else {
        return;
    };
    events.push(SessionEvent::tool_decision_recorded(evidence));
}

pub(crate) enum PendingExecutionOutcome {
    NativeObservation(ToolObservation),
    #[cfg(debug_assertions)]
    Applied {
        terminal_evidence: Option<TerminalEvidence>,
        response_evidence: Option<String>,
    },
    #[cfg(debug_assertions)]
    RecoverableFailure {
        reason: String,
        terminal_evidence: Option<TerminalEvidence>,
        response_evidence: Option<String>,
    },
    Failed(String),
}

impl PendingExecutionOutcome {
    #[cfg(debug_assertions)]
    fn recoverable(reason: impl Into<String>) -> Self {
        Self::RecoverableFailure {
            reason: reason.into(),
            terminal_evidence: None,
            response_evidence: None,
        }
    }

    #[cfg(debug_assertions)]
    fn execution_failure(
        reason: impl Into<String>,
        terminal_evidence: Option<TerminalEvidence>,
        response_evidence: Option<String>,
    ) -> Self {
        Self::RecoverableFailure {
            reason: reason.into(),
            terminal_evidence,
            response_evidence,
        }
    }
}

pub(crate) struct ClaimedAgentAction {
    approval_id: String,
    workspace_id: String,
    backend_id: String,
    workspace: WorkspaceRecord,
    process_registry: SharedProcessRegistry,
    mcp_runtime: desktoplab_tool_gateway::SharedMcpRuntime,
    pending: PendingAgentAction,
    native_call: Option<IterativeToolCall>,
    approval_mode: desktoplab_policy::ApprovalMode,
}

impl ClaimedAgentAction {
    #[must_use]
    pub(crate) fn execute(self) -> CompletedAgentAction {
        let outcome = match self.native_call.as_ref() {
            Some(call) => execute_native_approved_action_at(
                &self.workspace,
                &self.pending,
                call,
                self.approval_mode,
                &self.process_registry,
                &self.mcp_runtime,
            ),
            #[cfg(debug_assertions)]
            None => LocalApiRouter::execute_pending_agent_action_at(
                &self.workspace,
                &self.pending,
                &self.process_registry,
                &self.mcp_runtime,
            ),
            #[cfg(not(debug_assertions))]
            None => {
                PendingExecutionOutcome::Failed("legacy_pending_action_invalidated".to_string())
            }
        };
        CompletedAgentAction {
            approval_id: self.approval_id,
            workspace_id: self.workspace_id,
            backend_id: self.backend_id,
            outcome,
        }
    }
}

fn execute_native_approved_action_at(
    workspace: &WorkspaceRecord,
    pending: &PendingAgentAction,
    call: &IterativeToolCall,
    approval_mode: desktoplab_policy::ApprovalMode,
    process_registry: &SharedProcessRegistry,
    mcp_runtime: &desktoplab_tool_gateway::SharedMcpRuntime,
) -> PendingExecutionOutcome {
    if let Err(reason) = validate_native_approved_preconditions(workspace, pending) {
        return PendingExecutionOutcome::NativeObservation(ToolObservation::failure(call, reason));
    }
    let executor = CanonicalAgentToolExecutor::new(
        Path::new(&workspace.root_path),
        &workspace.workspace_id,
        pending.session_id(),
        CanonicalExecutionApproval::Approved,
    )
    .with_approval_mode(approval_mode)
    .with_process_registry(process_registry.clone())
    .with_mcp_runtime(mcp_runtime.clone());
    let observation = match executor {
        Ok(mut executor) => executor
            .execute_approved(call)
            .unwrap_or_else(|reason| ToolObservation::failure(call, reason)),
        Err(reason) => ToolObservation::failure(call, reason),
    };
    PendingExecutionOutcome::NativeObservation(observation)
}

fn validate_native_approved_preconditions(
    workspace: &WorkspaceRecord,
    pending: &PendingAgentAction,
) -> Result<(), String> {
    if !matches!(pending.tool(), ToolIntent::GitCommit { .. }) {
        return Ok(());
    }
    let root = Path::new(&workspace.root_path);
    let repo = GitRepository::open(root).map_err(|_| "git_repository_required".to_string())?;
    let status = repo.status().map_err(|error| error.to_string())?;
    let diff = repo.diff().map_err(|error| error.to_string())?;
    let fingerprint = git_change_fingerprint(status.entries(), diff.as_text());
    match pending.approved_change_fingerprint() {
        Some(approved) if approved == fingerprint => Ok(()),
        Some(_) => Err("working_tree_changed_after_approval".to_string()),
        None => Err("missing_approved_change_fingerprint".to_string()),
    }
}

pub(crate) struct CompletedAgentAction {
    approval_id: String,
    workspace_id: String,
    backend_id: String,
    outcome: PendingExecutionOutcome,
}

#[cfg(debug_assertions)]
fn execute_multi_file_patch(
    root: &Path,
    files: &[PendingMultiFilePatch],
) -> PendingExecutionOutcome {
    let items = files
        .iter()
        .map(|file| BatchPatchItem {
            path: file.path().to_string(),
            expected: file.expected().to_string(),
            replacement: file.replacement().to_string(),
        })
        .collect::<Vec<_>>();
    match FilesystemBatchPatchExecutor::new(root).apply(&items) {
        BatchPatchOutcome::Applied => {}
        BatchPatchOutcome::Conflict(path) => {
            return PendingExecutionOutcome::Failed(format!(
                "multi_file_patch_expected_content_missing:{path}"
            ));
        }
        BatchPatchOutcome::Blocked(reason) => {
            return PendingExecutionOutcome::Failed(format!(
                "multi_file_patch_capability_blocked:{reason}"
            ));
        }
    }

    PendingExecutionOutcome::Applied {
        terminal_evidence: None,
        response_evidence: Some(multi_file_patch_diff_evidence(files)),
    }
}

#[cfg(debug_assertions)]
fn multi_file_patch_diff_evidence(files: &[PendingMultiFilePatch]) -> String {
    let changed = files
        .iter()
        .map(PendingMultiFilePatch::path)
        .collect::<Vec<_>>()
        .join(", ");
    let summaries = files
        .iter()
        .map(|file| {
            format!(
                "{} expected_bytes={} replacement_bytes={}",
                file.path(),
                file.expected().len(),
                file.replacement().len()
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("Multi-file patch applied: {changed}\nPatch summaries: {summaries}")
}

#[cfg(debug_assertions)]
fn pending_changed_paths(pending: &PendingAgentAction) -> Vec<String> {
    pending
        .content()
        .and_then(pending_multi_file_patch_payload)
        .map(|files| {
            files
                .into_iter()
                .map(|file| file.path().to_string())
                .collect()
        })
        .unwrap_or_else(|| match pending.tool() {
            ToolIntent::FilesystemWrite { path } | ToolIntent::FilesystemPatch { path } => {
                vec![path.to_string()]
            }
            ToolIntent::FilesystemCreateDirectory { path }
            | ToolIntent::FilesystemDelete { path, .. } => vec![path.to_string()],
            ToolIntent::FilesystemMove {
                source,
                destination,
            } => vec![source.to_string(), destination.to_string()],
            _ => Vec::new(),
        })
}

#[cfg(debug_assertions)]
fn execute_pending_filesystem_mutation(root: &Path, tool: &ToolIntent) -> PendingExecutionOutcome {
    let mut executor = FilesystemMutationExecutor::new(root, PolicyEngine::default_conservative());
    let outcome = match tool {
        ToolIntent::FilesystemCreateDirectory { path } => {
            executor.create_directory(path, FilesystemApproval::Approved)
        }
        ToolIntent::FilesystemMove {
            source,
            destination,
        } => executor.move_path(source, destination, FilesystemApproval::Approved),
        ToolIntent::FilesystemDelete { path, recursive } => {
            executor.delete_path(path, *recursive, FilesystemApproval::Approved)
        }
        _ => return PendingExecutionOutcome::Failed("unexpected_filesystem_mutation".to_string()),
    };
    match outcome {
        FilesystemMutationOutcome::Changed => PendingExecutionOutcome::Applied {
            terminal_evidence: None,
            response_evidence: None,
        },
        FilesystemMutationOutcome::Unchanged => {
            PendingExecutionOutcome::recoverable("filesystem_no_change")
        }
        FilesystemMutationOutcome::ApprovalRequired => PendingExecutionOutcome::Failed(
            "filesystem mutation approval was not applied".to_string(),
        ),
        FilesystemMutationOutcome::Denied => {
            PendingExecutionOutcome::Failed("filesystem mutation denied".to_string())
        }
        FilesystemMutationOutcome::Blocked(reason) => PendingExecutionOutcome::Failed(reason),
    }
}

#[cfg(debug_assertions)]
fn terminal_evidence(
    command: &str,
    working_directory: &str,
    duration_ms: u128,
    result: &desktoplab_tool_gateway::TerminalExecutionResult,
) -> TerminalEvidence {
    let (status, exit_code) = match result.status() {
        TerminalExecutionStatus::Exited(code) => (format!("exited:{code}"), Some(code)),
        TerminalExecutionStatus::TimedOut => ("timed_out".to_string(), None),
        TerminalExecutionStatus::FailedToSpawn => ("failed_to_spawn".to_string(), None),
    };
    let output = format!(
        "status={status} duration_ms={duration_ms} cwd={} redaction_status={} redaction_count={} redaction_sources={}\nstdout:\n{}\nstderr:\n{}",
        working_directory,
        terminal_redaction_status(result),
        terminal_redaction_count(result),
        terminal_redaction_sources(result).join(","),
        result.stdout(),
        result.stderr()
    );
    TerminalEvidence::new(command, output, exit_code)
}

#[cfg(debug_assertions)]
fn execution_status_failure(status: &TerminalExecutionStatus, tests: bool) -> Option<String> {
    match status {
        TerminalExecutionStatus::Exited(0) => None,
        TerminalExecutionStatus::Exited(code) if tests => Some(format!("tests_failed:{code}")),
        TerminalExecutionStatus::Exited(code) => Some(format!("command_exit_nonzero:{code}")),
        TerminalExecutionStatus::TimedOut => Some("command_timed_out".to_string()),
        TerminalExecutionStatus::FailedToSpawn => Some("command_failed_to_spawn".to_string()),
    }
}

#[cfg(debug_assertions)]
fn test_terminal_evidence(
    evidence: &desktoplab_tool_gateway::TestRunEvidence,
    workspace_root: &str,
) -> TerminalEvidence {
    let exit_code = match evidence.status() {
        TerminalExecutionStatus::Exited(code) => Some(code),
        TerminalExecutionStatus::TimedOut | TerminalExecutionStatus::FailedToSpawn => None,
    };
    TerminalEvidence::new(
        evidence.command(),
        workspace_relative_evidence(&evidence.summary(), workspace_root),
        exit_code,
    )
}

#[cfg(debug_assertions)]
fn workspace_relative_evidence(evidence: &str, workspace_root: &str) -> String {
    let mut normalized = evidence.to_string();
    if let Ok(canonical_root) = fs::canonicalize(workspace_root) {
        if let Some(canonical_root) = canonical_root.to_str() {
            normalized = normalized.replace(canonical_root, ".");
        }
    }
    normalized.replace(workspace_root, ".")
}

#[cfg(debug_assertions)]
fn terminal_redaction_status(
    result: &desktoplab_tool_gateway::TerminalExecutionResult,
) -> &'static str {
    if terminal_redaction_count(result) > 0 {
        "redacted"
    } else {
        "clean"
    }
}

#[cfg(debug_assertions)]
fn terminal_redaction_count(result: &desktoplab_tool_gateway::TerminalExecutionResult) -> usize {
    result.stdout().matches("[REDACTED]").count() + result.stderr().matches("[REDACTED]").count()
}

#[cfg(debug_assertions)]
fn terminal_redaction_sources(
    result: &desktoplab_tool_gateway::TerminalExecutionResult,
) -> Vec<&'static str> {
    let mut sources = Vec::new();
    if result.stdout().contains("[REDACTED]") {
        sources.push("terminal.stdout");
    }
    if result.stderr().contains("[REDACTED]") {
        sources.push("terminal.stderr");
    }
    sources
}

#[cfg(debug_assertions)]
fn tool_requests_clarification(tool: Option<&ToolIntent>) -> bool {
    matches!(tool, Some(ToolIntent::Clarify { .. }))
}

#[cfg(debug_assertions)]
fn tool_has_immediate_observation(tool: Option<&ToolIntent>) -> bool {
    matches!(
        tool,
        Some(
            ToolIntent::FilesystemList { .. }
                | ToolIntent::FilesystemRead { .. }
                | ToolIntent::SearchText { .. }
                | ToolIntent::GitStatus
                | ToolIntent::GitDiff { .. }
                | ToolIntent::ProcessPoll { .. }
                | ToolIntent::ProcessStdin { .. }
                | ToolIntent::ProcessKill { .. }
                | ToolIntent::CreateCheckpoint { .. }
                | ToolIntent::Clarify { .. }
        )
    )
}

#[cfg(debug_assertions)]
fn equivalent_read_only_action(previous: &ToolIntent, next: &ToolIntent) -> bool {
    match (previous, next) {
        (
            ToolIntent::FilesystemList {
                path: previous_path,
            },
            ToolIntent::FilesystemList { path: next_path },
        ) => equivalent_workspace_path(previous_path.as_deref(), next_path.as_deref()),
        (
            ToolIntent::SearchText {
                query: previous_query,
                path: previous_path,
            },
            ToolIntent::SearchText {
                query: next_query,
                path: next_path,
            },
        ) => {
            previous_query == next_query
                && equivalent_workspace_path(previous_path.as_deref(), next_path.as_deref())
        }
        _ => previous == next,
    }
}

#[cfg(debug_assertions)]
fn equivalent_workspace_path(previous: Option<&str>, next: Option<&str>) -> bool {
    fn canonical(path: Option<&str>) -> Option<&str> {
        match path.map(str::trim) {
            None | Some("" | "." | "./" | "/" | "\\") => None,
            Some(path) => Some(path),
        }
    }

    canonical(previous) == canonical(next)
}

#[cfg(debug_assertions)]
fn clarification_targets_approval_gated_action(tool: &ToolIntent) -> bool {
    matches!(
        tool,
        ToolIntent::Clarify {
            blocked_action: Some(action),
            ..
        } if matches!(
            action.as_str(),
            "desktoplab.write_file"
                | "desktoplab.patch_file"
                | "desktoplab.create_directory"
                | "desktoplab.move_path"
                | "desktoplab.delete_path"
                | "desktoplab.run_terminal"
                | "desktoplab.start_process"
                | "desktoplab.run_tests"
                | "desktoplab.commit_changes"
                | "desktoplab.push_changes"
        )
    )
}

#[cfg(debug_assertions)]
fn clarification_repeats_observed_read(
    clarification: &ToolIntent,
    observed_tool: &ToolIntent,
) -> bool {
    let ToolIntent::Clarify {
        blocked_action: Some(action),
        ..
    } = clarification
    else {
        return false;
    };
    matches!(
        (action.as_str(), observed_tool),
        ("desktoplab.list_files", ToolIntent::FilesystemList { .. })
            | ("desktoplab.read_file", ToolIntent::FilesystemRead { .. })
            | ("desktoplab.search_text", ToolIntent::SearchText { .. })
            | ("desktoplab.git_status", ToolIntent::GitStatus)
            | ("desktoplab.git_diff", ToolIntent::GitDiff { .. })
    )
}

#[cfg(debug_assertions)]
fn clarification_is_optional_after_mutation(
    clarification: &ToolIntent,
    completed_tool: &ToolIntent,
) -> bool {
    match clarification {
        ToolIntent::Clarify {
            blocked_action: None,
            ..
        } => true,
        ToolIntent::Clarify {
            blocked_action: Some(action),
            ..
        } => {
            matches!(
                action.as_str(),
                "desktoplab.commit_changes" | "desktoplab.push_changes"
            ) || matches!(
                (completed_tool, action.as_str()),
                (
                    ToolIntent::FilesystemWrite { .. }
                        | ToolIntent::FilesystemPatch { .. }
                        | ToolIntent::FilesystemCreateDirectory { .. }
                        | ToolIntent::FilesystemMove { .. }
                        | ToolIntent::FilesystemDelete { .. },
                    "desktoplab.write_file"
                        | "desktoplab.patch_file"
                        | "desktoplab.create_directory"
                        | "desktoplab.move_path"
                        | "desktoplab.delete_path"
                ) | (ToolIntent::Terminal { .. }, "desktoplab.run_terminal")
                    | (ToolIntent::TestRun { .. }, "desktoplab.run_tests")
            )
        }
        _ => false,
    }
}

fn is_filesystem_mutation(tool: &ToolIntent) -> bool {
    filesystem_mutation_path(tool).is_some()
}

fn filesystem_mutation_path(tool: &ToolIntent) -> Option<&str> {
    match tool {
        ToolIntent::FilesystemWrite { path }
        | ToolIntent::FilesystemPatch { path }
        | ToolIntent::FilesystemCreateDirectory { path }
        | ToolIntent::FilesystemDelete { path, .. } => Some(path),
        ToolIntent::FilesystemMove { source, .. } => Some(source),
        _ => None,
    }
}

#[cfg(debug_assertions)]
fn repeats_completed_filesystem_target(completed: &ToolIntent, next: &ToolIntent) -> bool {
    filesystem_mutation_path(completed)
        .is_some_and(|completed_path| filesystem_mutation_path(next) == Some(completed_path))
}

#[cfg(debug_assertions)]
fn looks_like_unrecognized_tool_output(response: &str) -> bool {
    let compact = response.replace(char::is_whitespace, "");
    compact.contains("\"arguments\":")
        && (compact.contains("\"tool\":")
            || compact.contains("\"name\":")
            || compact.contains("\"function\":"))
        || compact.contains("\"desktoplabAction\":")
}

#[cfg(debug_assertions)]
fn provider_output_requires_initial_retry(response: &str) -> bool {
    if response.trim().is_empty() {
        return true;
    }
    if structured_clarification_missing_blocked_action(response) {
        return true;
    }
    looks_like_unrecognized_tool_output(response)
        && structured_action_tool(response).is_none()
        && structured_completion_message(response).is_none()
}

#[cfg(test)]
mod provider_output_retry_tests {
    use super::{agent_backend_prompt, provider_output_requires_initial_retry};
    use desktoplab_tool_gateway::ToolIntent;

    #[test]
    fn parseable_fenced_tool_calls_are_not_discarded_for_retry() {
        let read = "```json\n{\"name\":\"desktoplab.read_file\",\"arguments\":{\"path\":\"notes.md\"}}\n```";
        let complete = "```json\n{\"name\":\"desktoplab.complete\",\"arguments\":{\"message\":\"Done.\"}}\n```";

        assert!(!provider_output_requires_initial_retry(read));
        assert!(!provider_output_requires_initial_retry(complete));
    }

    #[test]
    fn malformed_tool_like_output_still_requires_retry() {
        assert!(provider_output_requires_initial_retry(
            r#"{"name":"desktoplab.read_file","arguments":{"path":"notes.md""#,
        ));
    }

    #[test]
    fn empty_provider_output_requires_retry() {
        assert!(provider_output_requires_initial_retry(""));
        assert!(provider_output_requires_initial_retry(" \n\t "));
    }

    #[test]
    fn clarification_without_a_canonical_blocked_action_requires_retry() {
        let incomplete = r#"{"name":"desktoplab.clarify","arguments":{"question":"What specific action should I perform next?"}}"#;
        let actionable = r#"{"name":"desktoplab.clarify","arguments":{"question":"Which replacement value should I use?","blockedOn":"desktoplab.patch_file"}}"#;

        assert!(provider_output_requires_initial_retry(incomplete));
        assert!(!provider_output_requires_initial_retry(actionable));
    }

    #[test]
    fn filesystem_prompt_uses_canonical_write_and_patch_semantics() {
        let prompt = agent_backend_prompt(
            "Update notes.md without replacing unrelated content.",
            Some(&ToolIntent::filesystem_write("notes.md")),
            true,
        );

        assert!(prompt.contains("Use desktoplab.write_file for a new file"));
        assert!(prompt.contains("use desktoplab.patch_file"));
        assert!(prompt.contains("complete content requested"));
        assert!(!prompt.contains("desktoplabAction"));
        assert!(!prompt.contains("create_file"));
    }

    #[test]
    fn generic_agent_prompt_requires_one_tool_call_per_turn() {
        let prompt = agent_backend_prompt("Inspect README.md and summarize it.", None, false);

        assert!(prompt.contains("exactly one tool call per model turn"));
        assert!(prompt.contains("wait for its executor observation"));
    }
}

#[cfg(debug_assertions)]
fn workspace_file_list_observation(
    root: &Path,
    path_prefix: Option<&str>,
) -> Result<String, String> {
    let path_prefix = normalized_workspace_prefix(path_prefix);
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(256, 24, 16_384));
    let entries = search
        .list_files(root)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|entry| {
            path_prefix
                .as_deref()
                .is_none_or(|prefix| entry.path().starts_with(prefix))
        })
        .take(24)
        .map(|entry| entry.path().to_string())
        .collect::<Vec<_>>();
    Ok(format!("Workspace files:\n{}", entries.join("\n")))
}

#[cfg(debug_assertions)]
fn workspace_search_observation(
    root: &Path,
    query: &str,
    path_prefix: Option<&str>,
) -> Result<String, String> {
    let path_prefix = normalized_workspace_prefix(path_prefix);
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(256, 24, 16_384));
    let report = search
        .search(root, query)
        .map_err(|error| error.to_string())?;
    let lines = report
        .matches()
        .iter()
        .filter(|hit| {
            path_prefix
                .as_deref()
                .is_none_or(|prefix| hit.path().starts_with(prefix))
        })
        .take(24)
        .map(|hit| format!("{}: {}", hit.path(), hit.preview()))
        .collect::<Vec<_>>();
    let suffix = if report.truncated() {
        "\n[truncated]"
    } else {
        ""
    };
    Ok(format!(
        "Search results for `{query}`:\n{}{suffix}",
        lines.join("\n")
    ))
}

#[cfg(debug_assertions)]
fn normalized_workspace_prefix(path_prefix: Option<&str>) -> Option<String> {
    let prefix = path_prefix?.trim();
    let prefix = prefix.strip_prefix("./").unwrap_or(prefix);
    let prefix = prefix.trim_matches('/');
    (!prefix.is_empty() && prefix != ".").then(|| prefix.to_string())
}

#[cfg(debug_assertions)]
fn stable_label_fragment(label: &str) -> String {
    let mut fragment = label
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>();
    while fragment.contains("--") {
        fragment = fragment.replace("--", "-");
    }
    let fragment = fragment.trim_matches('-');
    if fragment.is_empty() {
        "checkpoint".to_string()
    } else {
        fragment.chars().take(32).collect()
    }
}

fn git_commit_pending_content(tool: &ToolIntent, workspace_root: &Path) -> Option<String> {
    let ToolIntent::GitCommit { paths, .. } = tool else {
        return None;
    };
    let repo = GitRepository::open(workspace_root).ok()?;
    let status = repo.status().ok()?;
    let changed_files = status
        .files()
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();
    let selected_files = if paths.is_empty() {
        changed_files
    } else {
        normalized_file_set(paths)
    };
    Some(json!({ "changedFiles": selected_files }).to_string())
}

fn git_commit_content_has_no_changes(tool: &ToolIntent, content: Option<&str>) -> bool {
    matches!(tool, ToolIntent::GitCommit { .. })
        && pending_git_commit_changed_files(content).is_some_and(|files| files.is_empty())
}

fn pending_git_commit_changed_files(content: Option<&str>) -> Option<Vec<String>> {
    let value = serde_json::from_str::<serde_json::Value>(content?).ok()?;
    Some(
        value
            .get("changedFiles")?
            .as_array()?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect(),
    )
}

fn normalized_file_set(files: &[String]) -> Vec<String> {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    files
}

#[cfg(debug_assertions)]
fn repeats_completed_git_transition(completed: &ToolIntent, next: &ToolIntent) -> bool {
    matches!(
        (completed, next),
        (
            ToolIntent::GitCommit { message: completed, paths: completed_paths },
            ToolIntent::GitCommit { message: next, paths: next_paths }
        ) if completed == next && normalized_file_set(completed_paths) == normalized_file_set(next_paths)
    ) || matches!(
        (completed, next),
        (
            ToolIntent::GitPush {
                remote: completed_remote,
                branch: completed_branch,
            },
            ToolIntent::GitPush {
                remote: next_remote,
                branch: next_branch,
            }
        ) if completed_remote == next_remote && completed_branch == next_branch
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AgentApprovalRequestOutcome {
    Created,
    Deduplicated,
    CheckpointBlocked,
    #[cfg(debug_assertions)]
    Malformed,
    Skipped,
    #[cfg(debug_assertions)]
    PersistenceFailed,
}

enum CheckpointRequestOutcome {
    Ready(PendingAgentAction),
    Blocked(&'static str),
}

fn requires_checkpoint_before_approval(tool: &ToolIntent) -> bool {
    matches!(
        tool,
        ToolIntent::Terminal { .. } | ToolIntent::ProcessStart { .. }
    )
}

fn approved_change_fingerprint_for_tool(
    tool: &ToolIntent,
    workspace_root: &Path,
) -> Option<String> {
    if !matches!(tool, ToolIntent::GitCommit { .. }) {
        return None;
    }
    let repo = GitRepository::open(workspace_root).ok()?;
    let status_entries = repo.status().ok()?.entries().to_vec();
    let diff_text = repo
        .diff()
        .map(|diff| diff.as_text().to_string())
        .unwrap_or_default();
    Some(git_change_fingerprint(&status_entries, &diff_text))
}

#[cfg(debug_assertions)]
fn tool_decision_message(state: &str, tool: &ToolIntent) -> String {
    format!(
        "state={state} source={} canonical={} tool={} approval_mode=require_approval",
        tool_source(tool),
        tool.canonical_tool_id(),
        tool_evidence(tool)
    )
}

#[cfg(debug_assertions)]
fn tool_evidence(tool: &ToolIntent) -> String {
    tool.telemetry_evidence()
}

#[cfg(debug_assertions)]
fn tool_source(tool: &ToolIntent) -> &'static str {
    tool.telemetry_source()
}

fn context_paths_with_tool(context_paths: &[String], tool: Option<&ToolIntent>) -> Vec<String> {
    let mut paths = context_paths.to_vec();
    if let Some(path) = tool_context_path(tool)
        && !paths.iter().any(|existing| existing == path)
    {
        paths.push(path.to_string());
    }
    paths
}

fn tool_context_path(tool: Option<&ToolIntent>) -> Option<&str> {
    match tool? {
        ToolIntent::FilesystemRead { path }
        | ToolIntent::FilesystemWrite { path }
        | ToolIntent::FilesystemPatch { path }
        | ToolIntent::FilesystemCreateDirectory { path }
        | ToolIntent::FilesystemDelete { path, .. } => Some(path),
        ToolIntent::FilesystemMove { source, .. } => Some(source),
        ToolIntent::FilesystemList { path } | ToolIntent::SearchText { path, .. } => {
            path.as_deref()
        }
        ToolIntent::GitStatus
        | ToolIntent::GitDiff { .. }
        | ToolIntent::ProcessStart { .. }
        | ToolIntent::ProcessPoll { .. }
        | ToolIntent::ProcessStdin { .. }
        | ToolIntent::ProcessKill { .. }
        | ToolIntent::Terminal { .. }
        | ToolIntent::TestRun { .. }
        | ToolIntent::GitCommit { .. }
        | ToolIntent::GitPush { .. }
        | ToolIntent::CreateCheckpoint { .. }
        | ToolIntent::McpInvoke { .. }
        | ToolIntent::Clarify { .. }
        | ToolIntent::RuntimeInstall { .. } => None,
    }
}

#[cfg(debug_assertions)]
fn adapter_for_backend(backend_id: &str) -> LlmExecutionAdapter {
    if backend_id == "backend.codex" {
        LlmExecutionAdapter::provider(backend_id).with_provider_egress_allowed(true)
    } else {
        LlmExecutionAdapter::local(backend_id)
    }
}

#[cfg(debug_assertions)]
fn planned_tool(body: &str, tool_path: &str) -> Option<ToolIntent> {
    match body_field_or(body, "plannedTool", "").as_str() {
        "filesystem.list" | "desktoplab.list_files" => {
            Some(ToolIntent::filesystem_list(body_field(body, "path")))
        }
        "filesystem.read" | "desktoplab.read_file" => {
            Some(ToolIntent::filesystem_read(tool_path.to_string()))
        }
        "filesystem.write" | "desktoplab.write_file" => {
            Some(ToolIntent::filesystem_write(tool_path.to_string()))
        }
        "filesystem.patch" | "desktoplab.patch_file" => {
            Some(ToolIntent::filesystem_patch(tool_path.to_string()))
        }
        "search.text" | "desktoplab.search_text" => Some(ToolIntent::search_text(
            body_field(body, "query")?,
            body_field(body, "path"),
        )),
        "terminal.command" | "desktoplab.run_terminal" => {
            Some(ToolIntent::terminal(body_field(body, "command")?))
        }
        "desktoplab.run_tests" | "run_tests" | "test.run" => Some(ToolIntent::test_run(
            body_field(body, "command")?,
            body_field_or(body, "reason", "validate agent work"),
        )),
        "git.status" | "desktoplab.git_status" => Some(ToolIntent::git_status()),
        "git.diff" | "desktoplab.git_diff" => Some(ToolIntent::git_diff(body_field(body, "path"))),
        "git.commit" | "desktoplab.commit_changes" => Some(ToolIntent::git_commit_selected(
            body_field(body, "message")?,
            body_string_array(body, "paths"),
        )),
        "git.push" | "desktoplab.push_changes" => Some(ToolIntent::git_push(
            body_field(body, "remote")?,
            body_field(body, "branch")?,
        )),
        "checkpoint.create" | "desktoplab.create_checkpoint" => {
            Some(ToolIntent::create_checkpoint(body_field(body, "label")?))
        }
        "clarify" | "desktoplab.clarify" => {
            Some(ToolIntent::clarify(body_field(body, "question")?))
        }
        _ => None,
    }
}

fn has_planned_tool(body: &str) -> bool {
    body_field(body, "plannedTool").is_some()
}

fn planned_tool_test_harness_only_response() -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"PLANNED_TOOL_TEST_HARNESS_ONLY",
        "message":"plannedTool is reserved for DesktopLab test controls; normal sessions must use prompt intent or backend tool calls."
    }))
}

fn invalid_session_control_state(
    action: &str,
    state: desktoplab_agent_session::SessionState,
) -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"SESSION_CONTROL_INVALID_STATE",
        "message":"The requested control cannot run from the current session state.",
        "action":action,
        "state":format!("{state:?}").to_ascii_lowercase()
    }))
}

#[cfg(debug_assertions)]
fn planned_tool_missing_reason(body: &str) -> Option<&'static str> {
    match body_field_or(body, "plannedTool", "").as_str() {
        "terminal.command" if body_field(body, "command").is_none() => {
            Some("clarification_required:command")
        }
        "desktoplab.run_terminal" if body_field(body, "command").is_none() => {
            Some("clarification_required:command")
        }
        "search.text" | "desktoplab.search_text" if body_field(body, "query").is_none() => {
            Some("clarification_required:search_query")
        }
        "desktoplab.run_tests" | "run_tests" | "test.run"
            if body_field(body, "command").is_none() =>
        {
            Some("clarification_required:test_command")
        }
        "git.commit" | "desktoplab.commit_changes" if body_field(body, "message").is_none() => {
            Some("clarification_required:commit_message")
        }
        "git.push" | "desktoplab.push_changes"
            if body_field(body, "remote").is_none() || body_field(body, "branch").is_none() =>
        {
            Some("clarification_required:push_target")
        }
        "checkpoint.create" | "desktoplab.create_checkpoint"
            if body_field(body, "label").is_none() =>
        {
            Some("clarification_required:checkpoint_label")
        }
        "clarify" | "desktoplab.clarify" if body_field(body, "question").is_none() => {
            Some("clarification_required:question")
        }
        _ => None,
    }
}

fn agent_tool_approval_key(tool: Option<&ToolIntent>) -> Option<(String, String)> {
    match tool? {
        ToolIntent::FilesystemWrite { path } => Some((
            "filesystem.write".to_string(),
            format!("filesystem.write:{path}"),
        )),
        ToolIntent::FilesystemPatch { path } => Some((
            "filesystem.write".to_string(),
            format!("filesystem.patch:{path}"),
        )),
        ToolIntent::FilesystemCreateDirectory { path } => Some((
            "filesystem.write".to_string(),
            format!("filesystem.create_directory:{path}"),
        )),
        ToolIntent::FilesystemMove {
            source,
            destination,
        } => Some((
            "filesystem.write".to_string(),
            format!("filesystem.move:{source}:{destination}"),
        )),
        ToolIntent::FilesystemDelete { path, recursive } => Some((
            "filesystem.write".to_string(),
            format!("filesystem.delete:{path}:recursive={recursive}"),
        )),
        ToolIntent::Terminal { command, .. } => Some((
            "terminal.command".to_string(),
            format!("terminal:{command}"),
        )),
        ToolIntent::ProcessStart { command, .. } => Some((
            "terminal.command".to_string(),
            format!("process.start:{command}"),
        )),
        ToolIntent::TestRun { command, .. } => {
            Some(("test.run".to_string(), format!("test.run:{command}")))
        }
        ToolIntent::GitCommit { .. } => Some(("git.commit".to_string(), "git.commit".to_string())),
        ToolIntent::GitPush { .. } => Some(("git.push".to_string(), "git.push".to_string())),
        ToolIntent::McpInvoke { tool_id, .. } => Some((
            "mcp.tool.invoke".to_string(),
            format!("mcp.invoke:{tool_id}"),
        )),
        ToolIntent::FilesystemList { .. }
        | ToolIntent::FilesystemRead { .. }
        | ToolIntent::SearchText { .. }
        | ToolIntent::ProcessPoll { .. }
        | ToolIntent::ProcessStdin { .. }
        | ToolIntent::ProcessKill { .. }
        | ToolIntent::GitStatus
        | ToolIntent::GitDiff { .. }
        | ToolIntent::CreateCheckpoint { .. }
        | ToolIntent::Clarify { .. }
        | ToolIntent::RuntimeInstall { .. } => None,
    }
}

#[cfg(test)]
mod legacy_execution_guards_tests {
    use desktoplab_backend_services::ApprovalResolution;
    use desktoplab_tool_gateway::ToolIntent;

    use super::{
        LocalApiRouter, PendingAgentAction, PendingAgentActionState, PendingExecutionOutcome,
    };

    #[test]
    fn approved_action_without_its_session_fails_without_consuming_approval() {
        let mut router = LocalApiRouter::default();
        let seed = PendingAgentAction::new(
            "approval.seed",
            "session.missing",
            ToolIntent::filesystem_write("orphan.txt"),
            Some("must not be written".to_string()),
            true,
        );
        let approval = router.approvals.request_operation_with_payload_hash(
            "session.missing",
            "filesystem.write",
            "filesystem.write:orphan.txt",
            Some(seed.payload_hash()),
        );
        let pending = PendingAgentAction::new(
            approval.id(),
            "session.missing",
            ToolIntent::filesystem_write("orphan.txt"),
            Some("must not be written".to_string()),
            true,
        );
        router
            .agent_pending_actions
            .insert(approval.id().to_string(), pending);
        router
            .approvals
            .resolve(approval.id(), ApprovalResolution::Approve)
            .unwrap();

        assert!(router.claim_next_approved_agent_action().is_none());
        assert_eq!(
            router.agent_pending_actions[approval.id()].state(),
            PendingAgentActionState::Failed
        );
        assert!(!router.approvals.get(approval.id()).unwrap().is_consumed());
    }

    #[test]
    fn product_router_rejects_pre_canonical_pending_actions() {
        let router = LocalApiRouter::default();
        let pending = PendingAgentAction::new(
            "approval.old",
            "session.old",
            ToolIntent::filesystem_write("legacy.txt"),
            Some("must not be written".to_string()),
            true,
        );

        assert!(matches!(
            router.execute_approved_agent_action(&pending),
            PendingExecutionOutcome::Failed(reason)
                if reason == "legacy_pending_action_invalidated"
        ));
    }
}
