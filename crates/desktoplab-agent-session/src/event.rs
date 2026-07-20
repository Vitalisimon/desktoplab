use crate::{CheckpointRef, TerminalEvidence};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionEvent {
    Created {
        session_id: String,
        backend_id: String,
    },
    PlanningStarted {
        plan: String,
    },
    ExecutionStarted,
    CheckpointCreated {
        checkpoint: CheckpointRef,
    },
    Paused {
        reason: String,
    },
    Resumed,
    Blocked {
        reason: String,
    },
    BackendResponseReceived {
        message: String,
    },
    ToolDecisionRecorded {
        decision: String,
    },
    TestCommandProposed {
        command: String,
    },
    TerminalEvidenceRecorded {
        evidence: TerminalEvidence,
    },
    JobStarted {
        job_id: String,
        started_at: String,
        cancellable: bool,
    },
    JobHeartbeat {
        job_id: String,
        at: String,
    },
    JobObservation {
        job_id: String,
        message: String,
    },
    JobInterrupted {
        job_id: String,
        reason: String,
        guidance: String,
        at: String,
    },
    Failed {
        reason: String,
    },
    Cancelled {
        reason: String,
    },
    Completed {
        summary: String,
    },
}

impl SessionEvent {
    #[must_use]
    pub fn created(session_id: impl Into<String>, backend_id: impl Into<String>) -> Self {
        Self::Created {
            session_id: session_id.into(),
            backend_id: backend_id.into(),
        }
    }

    #[must_use]
    pub fn planning_started(plan: impl Into<String>) -> Self {
        Self::PlanningStarted { plan: plan.into() }
    }

    #[must_use]
    pub fn execution_started() -> Self {
        Self::ExecutionStarted
    }

    #[must_use]
    pub fn checkpoint_created(checkpoint: CheckpointRef) -> Self {
        Self::CheckpointCreated { checkpoint }
    }

    #[must_use]
    pub fn paused(reason: impl Into<String>) -> Self {
        Self::Paused {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn resumed() -> Self {
        Self::Resumed
    }

    #[must_use]
    pub fn cancelled(reason: impl Into<String>) -> Self {
        Self::Cancelled {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self::Blocked {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn backend_response_received(message: impl Into<String>) -> Self {
        Self::BackendResponseReceived {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn tool_decision_recorded(decision: impl Into<String>) -> Self {
        Self::ToolDecisionRecorded {
            decision: decision.into(),
        }
    }

    #[must_use]
    pub fn completed(summary: impl Into<String>) -> Self {
        Self::Completed {
            summary: summary.into(),
        }
    }

    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn test_command_proposed(command: impl Into<String>) -> Self {
        Self::TestCommandProposed {
            command: command.into(),
        }
    }

    #[must_use]
    pub fn terminal_evidence_recorded(evidence: TerminalEvidence) -> Self {
        Self::TerminalEvidenceRecorded { evidence }
    }

    #[must_use]
    pub fn job_started(
        job_id: impl Into<String>,
        started_at: impl Into<String>,
        cancellable: bool,
    ) -> Self {
        Self::JobStarted {
            job_id: job_id.into(),
            started_at: started_at.into(),
            cancellable,
        }
    }

    #[must_use]
    pub fn job_heartbeat(job_id: impl Into<String>, at: impl Into<String>) -> Self {
        Self::JobHeartbeat {
            job_id: job_id.into(),
            at: at.into(),
        }
    }

    #[must_use]
    pub fn job_observation(job_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::JobObservation {
            job_id: job_id.into(),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn job_interrupted(
        job_id: impl Into<String>,
        reason: impl Into<String>,
        guidance: impl Into<String>,
        at: impl Into<String>,
    ) -> Self {
        Self::JobInterrupted {
            job_id: job_id.into(),
            reason: reason.into(),
            guidance: guidance.into(),
            at: at.into(),
        }
    }
}
