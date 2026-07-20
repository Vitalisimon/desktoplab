use crate::{AgentSession, SessionEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    MissingCreatedEvent,
}

pub struct SessionReplay;

impl SessionReplay {
    pub fn replay(events: Vec<SessionEvent>) -> Result<AgentSession, ReplayError> {
        let mut iter = events.into_iter();
        let Some(SessionEvent::Created {
            session_id,
            backend_id,
        }) = iter.next()
        else {
            return Err(ReplayError::MissingCreatedEvent);
        };

        let mut session = AgentSession::new(session_id, backend_id);
        for event in iter {
            session.apply(event);
        }
        Ok(session)
    }
}
