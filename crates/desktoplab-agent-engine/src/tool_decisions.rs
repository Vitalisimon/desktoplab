use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_tool_gateway::ToolIntent;

use crate::loop_events::{tool_evidence, tool_source};

pub(crate) fn record(
    session: &mut AgentSession,
    events: &mut Vec<SessionEvent>,
    intent: &ToolIntent,
    state: &str,
    approval_mode: &str,
) {
    apply_event(
        session,
        events,
        SessionEvent::tool_decision_recorded(decision(intent, state, approval_mode)),
    );
}

pub(crate) fn record_with_reason(
    session: &mut AgentSession,
    events: &mut Vec<SessionEvent>,
    intent: &ToolIntent,
    state: &str,
    approval_mode: &str,
    reason: &str,
) {
    apply_event(
        session,
        events,
        SessionEvent::tool_decision_recorded(format!(
            "{} reason={reason}",
            decision(intent, state, approval_mode),
        )),
    );
}

fn decision(intent: &ToolIntent, state: &str, approval_mode: &str) -> String {
    format!(
        "state={state} source={} canonical={} tool={} approval_mode={approval_mode}",
        tool_source(intent),
        intent.canonical_tool_id(),
        tool_evidence(intent)
    )
}

fn apply_event(session: &mut AgentSession, events: &mut Vec<SessionEvent>, event: SessionEvent) {
    session.apply(event.clone());
    events.push(event);
}
