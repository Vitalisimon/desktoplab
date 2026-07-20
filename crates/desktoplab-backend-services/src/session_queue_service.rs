use crate::session_recovery::created_session_id;
use crate::session_turns::{SessionTurnQueue, SessionTurnSnapshot};
use crate::sessions::SessionService;

impl SessionService {
    pub fn enqueue_turn(
        &mut self,
        session_id: &str,
        prompt: impl Into<String>,
    ) -> Option<SessionTurnSnapshot> {
        self.with_turn_queue(session_id, |queue| queue.enqueue(prompt))
    }

    pub fn claim_next_turn(&mut self, session_id: &str) -> Option<SessionTurnSnapshot> {
        self.with_turn_queue(session_id, SessionTurnQueue::claim_next)
            .flatten()
    }

    pub fn complete_turn(&mut self, session_id: &str, turn_id: &str) -> bool {
        self.with_turn_queue(session_id, |queue| queue.complete(turn_id))
            .unwrap_or(false)
    }

    #[must_use]
    pub fn queued_turns(&self, session_id: &str) -> Vec<SessionTurnSnapshot> {
        self.turn_queue(session_id)
            .map(|queue| queue.snapshots())
            .unwrap_or_default()
    }

    pub fn request_cancel(&mut self, session_id: &str, requested_at_ms: u64, grace_ms: u64) {
        let _ = self.with_turn_queue(session_id, |queue| {
            queue.request_cancel(requested_at_ms, grace_ms)
        });
    }

    pub fn acknowledge_cancel(&mut self, session_id: &str) {
        let _ = self.with_turn_queue(session_id, SessionTurnQueue::acknowledge_cancel);
    }

    pub fn force_cancel_if_due(&mut self, session_id: &str, now_ms: u64) -> bool {
        self.with_turn_queue(session_id, |queue| queue.force_cancel_if_due(now_ms))
            .unwrap_or(false)
    }

    #[must_use]
    pub fn cancellation_state(&self, session_id: &str) -> Option<String> {
        self.turn_queue(session_id)
            .and_then(|queue| queue.cancellation_state().map(ToString::to_string))
    }

    fn with_turn_queue<T>(
        &mut self,
        session_id: &str,
        mutate: impl FnOnce(&mut SessionTurnQueue) -> T,
    ) -> Option<T> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("session store lock should not be poisoned");
        let record = data
            .records
            .iter_mut()
            .find(|record| created_session_id(&record.events) == Some(session_id))?;
        let result = mutate(&mut record.turn_queue);
        drop(data);
        self.store.persist();
        Some(result)
    }

    fn turn_queue(&self, session_id: &str) -> Option<SessionTurnQueue> {
        self.store
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .records
            .iter()
            .find(|record| created_session_id(&record.events) == Some(session_id))
            .map(|record| record.turn_queue.clone())
    }
}
