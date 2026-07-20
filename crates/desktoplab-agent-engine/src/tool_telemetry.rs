use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_redaction::redact_sensitive_bounded;
use desktoplab_tool_gateway::{TerminalRiskClass, ToolIntent, ToolOutcome};

use crate::ApprovalDecision;
use crate::loop_events::{tool_evidence, tool_source};

const TOOL_TELEMETRY_LIMIT: usize = 180;

pub(crate) fn record_before_tool(
    session: &mut AgentSession,
    events: &mut Vec<SessionEvent>,
    intent: &ToolIntent,
    outcome: &ToolOutcome,
    approval: ApprovalDecision,
    ordinal: usize,
    approval_mode: &str,
) {
    let tool = redact_sensitive_bounded(&tool_evidence(intent), TOOL_TELEMETRY_LIMIT);
    apply_event(
        session,
        events,
        SessionEvent::tool_decision_recorded(format!(
            "event=before_tool ordinal={ordinal} source={} canonical={} tool={} risk={} policy_result={} approval_state={} approval_mode={} redacted={}",
            tool_source(intent),
            intent.canonical_tool_id(),
            tool.value(),
            risk(intent),
            policy_result(outcome),
            approval_state(outcome, approval),
            approval_mode,
            tool.redacted()
        )),
    );
}

pub(crate) fn bounded_observation(observation: &str) -> String {
    redact_sensitive_bounded(observation, 800)
        .value()
        .to_string()
}

fn risk(intent: &ToolIntent) -> &'static str {
    match intent {
        ToolIntent::FilesystemList { .. }
        | ToolIntent::FilesystemRead { .. }
        | ToolIntent::SearchText { .. }
        | ToolIntent::GitStatus
        | ToolIntent::GitDiff { .. }
        | ToolIntent::ProcessPoll { .. }
        | ToolIntent::Clarify { .. } => "low",
        ToolIntent::CreateCheckpoint { .. }
        | ToolIntent::TestRun { .. }
        | ToolIntent::ProcessStdin { .. }
        | ToolIntent::ProcessKill { .. } => "medium",
        ToolIntent::Terminal { risk_class, .. } => terminal_risk(*risk_class),
        ToolIntent::FilesystemWrite { .. }
        | ToolIntent::FilesystemPatch { .. }
        | ToolIntent::FilesystemCreateDirectory { .. }
        | ToolIntent::FilesystemMove { .. }
        | ToolIntent::FilesystemDelete { .. }
        | ToolIntent::ProcessStart { .. }
        | ToolIntent::GitCommit { .. }
        | ToolIntent::GitPush { .. }
        | ToolIntent::McpInvoke { .. }
        | ToolIntent::RuntimeInstall { .. } => "high",
    }
}

fn terminal_risk(risk: TerminalRiskClass) -> &'static str {
    match risk {
        TerminalRiskClass::Low => "low",
        TerminalRiskClass::Medium => "medium",
        TerminalRiskClass::High => "high",
    }
}

fn policy_result(outcome: &ToolOutcome) -> &'static str {
    match outcome {
        ToolOutcome::Allowed(_) => "allowed",
        ToolOutcome::ApprovalRequired(_) => "requires_approval",
        ToolOutcome::Blocked(_) => "denied",
    }
}

fn approval_state(outcome: &ToolOutcome, approval: ApprovalDecision) -> &'static str {
    match (outcome, approval) {
        (ToolOutcome::Allowed(_), _) => "not_required",
        (ToolOutcome::Blocked(_), _) => "blocked",
        (ToolOutcome::ApprovalRequired(_), ApprovalDecision::Approved) => "approved",
        (ToolOutcome::ApprovalRequired(_), ApprovalDecision::Denied) => "denied",
        (ToolOutcome::ApprovalRequired(_), ApprovalDecision::Pending) => "pending",
    }
}

fn apply_event(session: &mut AgentSession, events: &mut Vec<SessionEvent>, event: SessionEvent) {
    session.apply(event.clone());
    events.push(event);
}
