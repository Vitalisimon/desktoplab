use std::{
    fmt,
    sync::{Arc, Mutex},
};

use desktoplab_agent_session::{
    AgentSession, SessionEvent, SessionReplay, SessionState, TerminalEvidence,
};
use desktoplab_storage::{
    ProductizationRecordKind, ProductizationStateRecord, SqliteStore, StorageError,
};

use crate::session_recovery::{
    SessionContinuation, SessionRecoverySnapshot, created_session_id, recovery_snapshot,
};
use crate::session_storage::{load_session_data, session_data_json};
use crate::session_trace::{SessionTraceEnvelope, SessionTraceEvent, append_trace_event};
use crate::session_turns::SessionTurnQueue;

#[derive(Clone, Debug)]
pub(crate) struct SessionRecord {
    pub(crate) workspace_id: String,
    pub(crate) events: Vec<SessionEvent>,
    pub(crate) trace: Vec<SessionTraceEvent>,
    pub(crate) turn_queue: SessionTurnQueue,
}

#[derive(Clone, Default)]
pub struct SessionServiceStore {
    pub(crate) inner: Arc<Mutex<SessionData>>,
    storage: Option<Arc<Mutex<SqliteStore>>>,
    persistence_fault: Arc<Mutex<Option<String>>>,
}

impl fmt::Debug for SessionServiceStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionServiceStore")
            .field("has_storage", &self.storage.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SessionData {
    pub(crate) next_session_number: u64,
    pub(crate) records: Vec<SessionRecord>,
}

impl SessionServiceStore {
    pub fn with_storage(storage: SqliteStore) -> Result<Self, StorageError> {
        let mut data = load_session_data(&storage)?;
        for record in &mut data.records {
            record.turn_queue.recover_interrupted();
        }
        let store = Self {
            inner: Arc::new(Mutex::new(data)),
            storage: Some(Arc::new(Mutex::new(storage))),
            persistence_fault: Arc::new(Mutex::new(None)),
        };
        store.persist_result()?;
        Ok(store)
    }

    pub(crate) fn persist(&self) {
        if let Err(error) = self.persist_result() {
            let mut fault = self
                .persistence_fault
                .lock()
                .expect("session persistence fault lock should not be poisoned");
            if fault.is_none() {
                *fault = Some(error.to_string());
            }
        }
    }

    fn persist_result(&self) -> Result<(), StorageError> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        let data = self
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .clone();
        let payload = session_data_json(&data).to_string();
        storage
            .lock()
            .expect("session storage lock should not be poisoned")
            .put_productization_state(ProductizationStateRecord::new(
                ProductizationRecordKind::AgentSession,
                "sessions",
                payload,
            ))
    }

    #[must_use]
    pub fn persistence_fault(&self) -> Option<String> {
        self.persistence_fault
            .lock()
            .expect("session persistence fault lock should not be poisoned")
            .clone()
    }
}

#[derive(Clone, Debug)]
pub struct SessionService {
    pub(crate) store: SessionServiceStore,
}

impl SessionService {
    #[must_use]
    pub fn new(store: SessionServiceStore) -> Self {
        Self { store }
    }

    #[must_use]
    pub fn persistence_fault(&self) -> Option<String> {
        self.store.persistence_fault()
    }

    pub fn create_session(
        &mut self,
        workspace_id: impl Into<String>,
        backend_id: impl Into<String>,
    ) -> AgentSession {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("session store lock should not be poisoned");
        data.next_session_number += 1;
        let session_id = format!("session.{}", data.next_session_number);
        let backend_id = backend_id.into();
        let event = SessionEvent::created(session_id.clone(), backend_id);
        let mut trace = Vec::new();
        append_trace_event(&session_id, &mut trace, &event);
        data.records.push(SessionRecord {
            workspace_id: workspace_id.into(),
            events: vec![event.clone()],
            trace,
            turn_queue: SessionTurnQueue::default(),
        });
        drop(data);
        self.store.persist();
        SessionReplay::replay(vec![event]).expect("created event should replay")
    }

    pub fn start(&mut self, session_id: &str) {
        self.append(session_id, SessionEvent::execution_started());
    }

    pub fn plan(&mut self, session_id: &str, plan: impl Into<String>) {
        self.append(session_id, SessionEvent::planning_started(plan));
    }

    pub fn pause(&mut self, session_id: &str, reason: impl Into<String>) {
        self.append(session_id, SessionEvent::paused(reason));
    }

    pub fn resume(&mut self, session_id: &str) {
        self.append(session_id, SessionEvent::resumed());
    }

    pub fn block(&mut self, session_id: &str, reason: impl Into<String>) {
        self.append(session_id, SessionEvent::blocked(reason));
    }

    pub fn fail(&mut self, session_id: &str, reason: impl Into<String>) {
        self.append(
            session_id,
            SessionEvent::Failed {
                reason: reason.into(),
            },
        );
    }

    pub fn cancel(&mut self, session_id: &str, reason: impl Into<String>) {
        self.append(session_id, SessionEvent::cancelled(reason));
    }

    pub fn complete(&mut self, session_id: &str, summary: impl Into<String>) {
        self.append(session_id, SessionEvent::completed(summary));
    }

    pub fn request_test_command(&mut self, session_id: &str, command: impl Into<String>) {
        self.append(session_id, SessionEvent::test_command_proposed(command));
        self.block(session_id, "terminal command approval required");
    }

    pub fn record_test_result(
        &mut self,
        session_id: &str,
        command: impl Into<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) {
        self.append(
            session_id,
            SessionEvent::terminal_evidence_recorded(TerminalEvidence::new(
                command, output, exit_code,
            )),
        );
    }

    pub fn append_events(&mut self, session_id: &str, events: &[SessionEvent]) {
        for event in events {
            if matches!(event, SessionEvent::Created { .. }) {
                continue;
            }
            self.append(session_id, event.clone());
        }
    }

    #[must_use]
    pub fn get(&self, session_id: &str) -> Option<AgentSession> {
        self.replay(session_id)
    }

    #[must_use]
    pub fn replay(&self, session_id: &str) -> Option<AgentSession> {
        self.store
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .records
            .iter()
            .find(|record| created_session_id(&record.events) == Some(session_id))
            .and_then(|record| SessionReplay::replay(record.events.clone()).ok())
    }

    #[must_use]
    pub fn trace(&self, session_id: &str) -> Option<SessionTraceEnvelope> {
        self.store
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .records
            .iter()
            .find(|record| created_session_id(&record.events) == Some(session_id))
            .map(|record| SessionTraceEnvelope::new(session_id, record.trace.clone()))
    }

    #[must_use]
    pub fn recover(&self, session_id: &str) -> Option<SessionRecoverySnapshot> {
        let data = self
            .store
            .inner
            .lock()
            .expect("session store lock should not be poisoned");
        data.records
            .iter()
            .find(|record| created_session_id(&record.events) == Some(session_id))
            .and_then(recovery_snapshot)
    }

    pub fn resume_after_approval(&mut self, session_id: &str) -> SessionContinuation {
        let Some(snapshot) = self.recover(session_id) else {
            return SessionContinuation::Missing;
        };
        match snapshot.session().state() {
            SessionState::Blocked | SessionState::Paused => {
                self.resume(session_id);
                SessionContinuation::Resumed
            }
            SessionState::Running => SessionContinuation::AlreadyRunning,
            SessionState::Failed | SessionState::Cancelled | SessionState::Completed => {
                SessionContinuation::Terminal(snapshot.session().state())
            }
            SessionState::Created | SessionState::Planning => SessionContinuation::AlreadyRunning,
        }
    }

    #[must_use]
    pub fn list_by_workspace(&self, workspace_id: &str) -> Vec<AgentSession> {
        self.store
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .records
            .iter()
            .filter(|record| record.workspace_id == workspace_id)
            .filter_map(|record| SessionReplay::replay(record.events.clone()).ok())
            .collect()
    }

    #[must_use]
    pub fn workspace_id_for(&self, session_id: &str) -> Option<String> {
        self.store
            .inner
            .lock()
            .expect("session store lock should not be poisoned")
            .records
            .iter()
            .find(|record| created_session_id(&record.events) == Some(session_id))
            .map(|record| record.workspace_id.clone())
    }

    pub(crate) fn append(&mut self, session_id: &str, event: SessionEvent) {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("session store lock should not be poisoned");
        if let Some(record) = data
            .records
            .iter_mut()
            .find(|record| created_session_id(&record.events) == Some(session_id))
        {
            append_trace_event(session_id, &mut record.trace, &event);
            record.events.push(event);
            drop(data);
            self.store.persist();
        }
    }
}
