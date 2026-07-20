use desktoplab_agent_session::SessionEvent;
use desktoplab_tool_gateway::ToolIntent;

pub(crate) fn tool_evidence(intent: &ToolIntent) -> String {
    intent.telemetry_evidence()
}

pub(crate) fn tool_source(intent: &ToolIntent) -> &'static str {
    intent.telemetry_source()
}

pub(crate) fn event_name(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::Created { .. } => "created",
        SessionEvent::PlanningStarted { .. } => "planning_started",
        SessionEvent::ExecutionStarted => "execution_started",
        SessionEvent::CheckpointCreated { .. } => "checkpoint_created",
        SessionEvent::Paused { .. } => "paused",
        SessionEvent::Resumed => "resumed",
        SessionEvent::Blocked { .. } => "blocked",
        SessionEvent::BackendResponseReceived { .. } => "backend_response_received",
        SessionEvent::ToolDecisionRecorded { .. } => "tool_decision",
        SessionEvent::TestCommandProposed { .. } => "test_command_proposed",
        SessionEvent::TerminalEvidenceRecorded { .. } => "terminal_evidence_recorded",
        SessionEvent::JobStarted { .. } => "job_started",
        SessionEvent::JobHeartbeat { .. } => "job_heartbeat",
        SessionEvent::JobObservation { .. } => "job_observation",
        SessionEvent::JobInterrupted { .. } => "job_interrupted",
        SessionEvent::Failed { .. } => "failed",
        SessionEvent::Cancelled { .. } => "cancelled",
        SessionEvent::Completed { .. } => "completed",
    }
}
