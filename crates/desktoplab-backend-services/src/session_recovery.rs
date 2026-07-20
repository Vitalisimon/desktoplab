use desktoplab_agent_session::{AgentSession, SessionEvent, SessionReplay, SessionState};

use crate::sessions::SessionRecord;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionContinuation {
    Resumed,
    AlreadyRunning,
    Terminal(SessionState),
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionCursor {
    event_count: usize,
    can_continue_after_approval: bool,
}

impl SessionCursor {
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    #[must_use]
    pub fn can_continue_after_approval(&self) -> bool {
        self.can_continue_after_approval
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecoverySnapshot {
    session: AgentSession,
    cursor: SessionCursor,
}

impl SessionRecoverySnapshot {
    #[must_use]
    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    #[must_use]
    pub fn cursor(&self) -> &SessionCursor {
        &self.cursor
    }
}

pub(crate) fn created_session_id(events: &[SessionEvent]) -> Option<&str> {
    let Some(SessionEvent::Created { session_id, .. }) = events.first() else {
        return None;
    };
    Some(session_id)
}

pub(crate) fn recovery_snapshot(record: &SessionRecord) -> Option<SessionRecoverySnapshot> {
    let session = SessionReplay::replay(record.events.clone()).ok()?;
    let can_continue_after_approval = matches!(
        session.state(),
        SessionState::Blocked | SessionState::Paused
    );
    Some(SessionRecoverySnapshot {
        session,
        cursor: SessionCursor {
            event_count: record.events.len(),
            can_continue_after_approval,
        },
    })
}
