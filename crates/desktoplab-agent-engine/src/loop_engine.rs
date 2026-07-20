use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_tool_gateway::ToolGateway;

use crate::loop_events::event_name;
use crate::output_sanitizer::sanitize_model_output;
use crate::{
    AgentRunRequest, BoundedToolLoop, LlmExecutionAdapter, LlmExecutionError, ToolLoopLimits,
    ToolLoopStep,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalDecision {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentEvidence {
    ApprovalDenied,
    BackendBlocked(String),
    ToolExecuted(String),
    DiffCaptured(String),
    TestExecuted(String),
}

pub struct AgentLoop {
    gateway: ToolGateway,
    approval: ApprovalDecision,
    backend_adapter: Option<LlmExecutionAdapter>,
}

impl AgentLoop {
    #[must_use]
    pub fn new(gateway: ToolGateway) -> Self {
        Self {
            gateway,
            approval: ApprovalDecision::Pending,
            backend_adapter: None,
        }
    }

    #[must_use]
    pub fn with_approval(mut self, approval: ApprovalDecision) -> Self {
        self.approval = approval;
        self
    }

    #[must_use]
    pub fn with_backend_adapter(mut self, adapter: LlmExecutionAdapter) -> Self {
        self.backend_adapter = Some(adapter);
        self
    }

    pub fn run(&mut self, request: AgentRunRequest) -> AgentRunResult {
        let mut session = AgentSession::new(request.session_id(), request.backend_id());
        let mut events = vec![SessionEvent::created(
            request.session_id(),
            request.backend_id(),
        )];
        apply_event(
            &mut session,
            &mut events,
            SessionEvent::planning_started(
                request.prompt().unwrap_or("plan accepted for execution"),
            ),
        );
        let backend_response = match self.backend_response(&request) {
            Ok(response) => response,
            Err(reason) => {
                let evidence = vec![AgentEvidence::BackendBlocked(reason.to_string())];
                let event = if reason == "local_inference_failed" {
                    SessionEvent::failed(reason)
                } else {
                    SessionEvent::blocked(reason)
                };
                apply_event(&mut session, &mut events, event);
                return AgentRunResult::new(session, 0, evidence, events);
            }
        };
        apply_event(
            &mut session,
            &mut events,
            SessionEvent::backend_response_received(backend_response),
        );
        apply_event(&mut session, &mut events, SessionEvent::execution_started());
        let steps = request
            .tool_calls()
            .iter()
            .map(ToolLoopStep::from_planned)
            .collect::<Vec<_>>();
        let mut tool_loop = BoundedToolLoop::new(self.gateway.clone(), ToolLoopLimits::default());
        let tool_result = tool_loop.run(&mut session, &mut events, &steps, self.approval);
        let pending_approvals = tool_result.pending_approvals();
        let mut evidence = tool_result.evidence().to_vec();
        if tool_result.stop_reason().is_some() {
            return AgentRunResult::new(session, pending_approvals, evidence, events);
        }

        if let Some(diff) = request.diff() {
            evidence.push(AgentEvidence::DiffCaptured(diff.to_string()));
        }
        if let Some(test_result) = request.test_result() {
            evidence.push(AgentEvidence::TestExecuted(test_result.to_string()));
        }
        apply_event(
            &mut session,
            &mut events,
            SessionEvent::completed("agent loop completed"),
        );
        AgentRunResult::new(session, pending_approvals, evidence, events)
    }

    fn backend_response(&self, request: &AgentRunRequest) -> Result<String, &'static str> {
        if let Some(response) = request.backend_response() {
            return Ok(sanitize_model_output(response));
        }
        let Some(adapter) = &self.backend_adapter else {
            return Err("backend_adapter_unavailable");
        };
        adapter
            .complete(
                request
                    .backend_prompt()
                    .or_else(|| request.prompt())
                    .unwrap_or_default(),
            )
            .map(|stream| sanitize_model_output(stream.output()))
            .map_err(|error| match error {
                LlmExecutionError::LocalInferenceNotConfigured => "local_inference_not_configured",
                LlmExecutionError::LocalInferenceFailed => "local_inference_failed",
                LlmExecutionError::ProviderEgressDenied => "provider_egress_denied",
                LlmExecutionError::ExternalBackendUnavailable => "external_backend_unavailable",
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunResult {
    session: AgentSession,
    pending_approvals: usize,
    evidence: Vec<AgentEvidence>,
    events: Vec<SessionEvent>,
}

impl AgentRunResult {
    #[must_use]
    pub fn new(
        session: AgentSession,
        pending_approvals: usize,
        evidence: Vec<AgentEvidence>,
        events: Vec<SessionEvent>,
    ) -> Self {
        Self {
            session,
            pending_approvals,
            evidence,
            events,
        }
    }

    #[must_use]
    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    #[must_use]
    pub fn pending_approvals(&self) -> usize {
        self.pending_approvals
    }

    #[must_use]
    pub fn evidence(&self) -> &[AgentEvidence] {
        &self.evidence
    }

    #[must_use]
    pub fn events(&self) -> &[SessionEvent] {
        &self.events
    }

    #[must_use]
    pub fn event_names(&self) -> Vec<&'static str> {
        self.events.iter().map(event_name).collect()
    }
}

fn apply_event(session: &mut AgentSession, events: &mut Vec<SessionEvent>, event: SessionEvent) {
    session.apply(event.clone());
    events.push(event);
}
