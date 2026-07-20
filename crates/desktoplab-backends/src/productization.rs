use desktoplab_agent_session::{AgentSession, SessionEvent};

use crate::{ExternalBackendHarness, ExternalEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeCallFailure {
    code: String,
    message: String,
}

impl BridgeCallFailure {
    #[must_use]
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedBridgeEvents {
    session: AgentSession,
    events: Vec<SessionEvent>,
    evidence: String,
}

impl ImportedBridgeEvents {
    #[must_use]
    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    #[must_use]
    pub fn events(&self) -> &[SessionEvent] {
        &self.events
    }

    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }
}

pub(crate) fn import_events(
    harness: &ExternalBackendHarness,
    session_id: &str,
    external_events: Vec<ExternalEvent>,
) -> ImportedBridgeEvents {
    let mut session = harness.create_session(session_id);
    let mut events = vec![SessionEvent::created(session_id, harness.backend_id())];
    for external_event in external_events {
        let event = harness.normalize_event(external_event);
        session.apply(event.clone());
        events.push(event);
    }
    ImportedBridgeEvents {
        session,
        events,
        evidence: "external_events_imported".to_string(),
    }
}

pub(crate) fn record_failure(
    harness: &ExternalBackendHarness,
    session_id: &str,
    failure: BridgeCallFailure,
) -> ImportedBridgeEvents {
    let mut session = harness.create_session(session_id);
    let blocked = SessionEvent::blocked(format!("{}:{}", failure.code, failure.message));
    session.apply(blocked.clone());
    ImportedBridgeEvents {
        session,
        events: vec![
            SessionEvent::created(session_id, harness.backend_id()),
            blocked,
        ],
        evidence: format!("{} {}", failure.code, failure.message),
    }
}

#[must_use]
pub(crate) fn redact_auth(value: &str) -> String {
    let mut out = value.to_string();
    for token in value.split_whitespace() {
        if token.starts_with("sk-") || token.contains("secret") || token.starts_with("Bearer") {
            out = out.replace(token, "[REDACTED]");
        }
    }
    out
}
