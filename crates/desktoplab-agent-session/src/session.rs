#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionOwner {
    DesktopLab,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    Created,
    Planning,
    Running,
    Paused,
    Blocked,
    Failed,
    Cancelled,
    Completed,
}

use crate::{AgentJobSnapshot, TerminalEvidence};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointRef(String);

impl CheckpointRef {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSession {
    session_id: String,
    execution_backend_id: String,
    owner: SessionOwner,
    state: SessionState,
    checkpoints: Vec<CheckpointRef>,
    backend_responses: Vec<String>,
    tool_decisions: Vec<String>,
    proposed_test_commands: Vec<String>,
    terminal_evidence: Vec<TerminalEvidence>,
    job: Option<AgentJobSnapshot>,
    event_log: Vec<crate::event::SessionEvent>,
    blocked_reason: Option<String>,
    failed_reason: Option<String>,
    plan: Option<String>,
    plans: Vec<String>,
    summary: Option<String>,
}

impl AgentSession {
    #[must_use]
    pub fn new(session_id: impl Into<String>, execution_backend_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            execution_backend_id: execution_backend_id.into(),
            owner: SessionOwner::DesktopLab,
            state: SessionState::Created,
            checkpoints: Vec::new(),
            backend_responses: Vec::new(),
            tool_decisions: Vec::new(),
            proposed_test_commands: Vec::new(),
            terminal_evidence: Vec::new(),
            job: None,
            event_log: Vec::new(),
            blocked_reason: None,
            failed_reason: None,
            plan: None,
            plans: Vec::new(),
            summary: None,
        }
    }

    pub fn apply(&mut self, event: SessionEvent) {
        self.event_log.push(event.clone());
        match event {
            SessionEvent::Created { .. } => {}
            SessionEvent::PlanningStarted { plan } => {
                self.state = SessionState::Planning;
                self.blocked_reason = None;
                self.plan = Some(plan.clone());
                self.plans.push(plan);
            }
            SessionEvent::ExecutionStarted | SessionEvent::Resumed => {
                self.state = SessionState::Running;
                self.blocked_reason = None;
                if let Some(job) = self.job.as_mut() {
                    job.resume();
                }
            }
            SessionEvent::CheckpointCreated { checkpoint } => self.checkpoints.push(checkpoint),
            SessionEvent::Paused { .. } => {
                self.state = SessionState::Paused;
                self.blocked_reason = None;
                if let Some(job) = self.job.as_mut() {
                    job.pause("paused");
                }
            }
            SessionEvent::Blocked { reason } => {
                self.state = SessionState::Blocked;
                self.blocked_reason = Some(reason);
                if let Some(job) = self.job.as_mut() {
                    job.pause("blocked");
                }
            }
            SessionEvent::BackendResponseReceived { message } => {
                self.backend_responses.push(message);
            }
            SessionEvent::ToolDecisionRecorded { decision } => {
                self.tool_decisions.push(decision);
            }
            SessionEvent::TestCommandProposed { command } => {
                self.proposed_test_commands.push(command);
            }
            SessionEvent::TerminalEvidenceRecorded { evidence } => {
                self.terminal_evidence.push(evidence);
            }
            SessionEvent::JobStarted {
                job_id,
                started_at,
                cancellable,
            } => {
                self.job = Some(AgentJobSnapshot::running(job_id, started_at, cancellable));
            }
            SessionEvent::JobHeartbeat { job_id: _, at } => {
                if let Some(job) = self.job.as_mut() {
                    job.heartbeat(at);
                }
            }
            SessionEvent::JobObservation { job_id: _, message } => {
                if let Some(job) = self.job.as_mut() {
                    job.observe(message);
                }
            }
            SessionEvent::JobInterrupted {
                job_id: _,
                reason,
                guidance,
                at: _,
            } => {
                if let Some(job) = self.job.as_mut() {
                    job.interrupt(guidance);
                }
                self.state = SessionState::Blocked;
                self.blocked_reason = Some(reason);
            }
            SessionEvent::Failed { reason } => {
                if let Some(job) = self.job.as_mut() {
                    job.fail();
                }
                self.state = SessionState::Failed;
                self.blocked_reason = None;
                self.failed_reason = Some(reason);
            }
            SessionEvent::Cancelled { .. } => {
                if let Some(job) = self.job.as_mut() {
                    job.cancel();
                }
                self.state = SessionState::Cancelled;
                self.blocked_reason = None;
            }
            SessionEvent::Completed { summary } => {
                if let Some(job) = self.job.as_mut() {
                    job.complete();
                }
                self.state = SessionState::Completed;
                self.blocked_reason = None;
                self.summary = Some(summary);
            }
        }
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn owner(&self) -> SessionOwner {
        self.owner
    }

    #[must_use]
    pub fn execution_backend_id(&self) -> &str {
        &self.execution_backend_id
    }

    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    #[must_use]
    pub fn checkpoints(&self) -> &[CheckpointRef] {
        &self.checkpoints
    }

    #[must_use]
    pub fn backend_responses(&self) -> &[String] {
        &self.backend_responses
    }

    #[must_use]
    pub fn tool_decisions(&self) -> &[String] {
        &self.tool_decisions
    }

    #[must_use]
    pub fn proposed_test_commands(&self) -> &[String] {
        &self.proposed_test_commands
    }

    #[must_use]
    pub fn terminal_evidence(&self) -> &[TerminalEvidence] {
        &self.terminal_evidence
    }

    #[must_use]
    pub fn job(&self) -> Option<&AgentJobSnapshot> {
        self.job.as_ref()
    }

    #[must_use]
    pub fn event_log(&self) -> &[SessionEvent] {
        &self.event_log
    }

    #[must_use]
    pub fn blocked_reason(&self) -> Option<&str> {
        self.blocked_reason.as_deref()
    }

    #[must_use]
    pub fn failed_reason(&self) -> Option<&str> {
        self.failed_reason.as_deref()
    }

    #[must_use]
    pub fn plan(&self) -> Option<&str> {
        self.plan.as_deref()
    }

    #[must_use]
    pub fn plans(&self) -> &[String] {
        &self.plans
    }

    #[must_use]
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
}
use crate::event::SessionEvent;
