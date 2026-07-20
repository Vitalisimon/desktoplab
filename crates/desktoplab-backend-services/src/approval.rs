use std::sync::{Arc, Mutex};

use crate::approval_terminal::{TerminalCommandApproval, terminal_approval};
use desktoplab_tool_gateway::TerminalCommandRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl ApprovalState {
    #[must_use]
    pub fn is_denied(self) -> bool {
        self == Self::Denied
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    #[must_use]
    pub fn from_stable_str(value: &str) -> Self {
        match value {
            "approved" => Self::Approved,
            "denied" => Self::Denied,
            "expired" => Self::Expired,
            _ => Self::Pending,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalResolution {
    Approve,
    Deny,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequestRecord {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) action: String,
    pub(crate) operation_id: String,
    pub(crate) payload_hash: Option<String>,
    pub(crate) consumed: bool,
    pub(crate) state: ApprovalState,
}

impl ApprovalRequestRecord {
    #[must_use]
    pub fn restored(
        id: impl Into<String>,
        session_id: impl Into<String>,
        action: impl Into<String>,
        operation_id: impl Into<String>,
        payload_hash: Option<String>,
        consumed: bool,
        state: ApprovalState,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            action: action.into(),
            operation_id: operation_id.into(),
            payload_hash,
            consumed,
            state,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn state(&self) -> ApprovalState {
        self.state
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn action(&self) -> &str {
        &self.action
    }

    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    #[must_use]
    pub fn payload_hash(&self) -> Option<&str> {
        self.payload_hash.as_deref()
    }

    #[must_use]
    pub fn is_consumed(&self) -> bool {
        self.consumed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionWaitState {
    Blocked { approval_id: String },
    Resumed,
}

impl SessionWaitState {
    #[must_use]
    pub fn blocked_on(approval_id: impl Into<String>) -> Self {
        Self::Blocked {
            approval_id: approval_id.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ApprovalStore {
    pub(crate) records: Arc<Mutex<Vec<ApprovalRequestRecord>>>,
}

impl ApprovalStore {
    fn insert(&self, record: ApprovalRequestRecord) {
        self.records
            .lock()
            .expect("approval store lock should not be poisoned")
            .push(record);
    }

    fn update_state(
        &self,
        approval_id: &str,
        state: ApprovalState,
    ) -> Option<ApprovalRequestRecord> {
        let mut records = self
            .records
            .lock()
            .expect("approval store lock should not be poisoned");
        let record = records.iter_mut().find(|record| record.id == approval_id)?;
        record.state = state;
        Some(record.clone())
    }

    fn get(&self, approval_id: &str) -> Option<ApprovalRequestRecord> {
        self.records
            .lock()
            .expect("approval store lock should not be poisoned")
            .iter()
            .find(|record| record.id == approval_id)
            .cloned()
    }

    fn list(&self) -> Vec<ApprovalRequestRecord> {
        self.records
            .lock()
            .expect("approval store lock should not be poisoned")
            .clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalAuditRecord {
    approval_id: String,
    event: String,
}

#[derive(Clone, Debug)]
pub struct ApprovalService {
    pub(crate) store: ApprovalStore,
    audit: Vec<ApprovalAuditRecord>,
    next_id: u64,
}

impl ApprovalService {
    #[must_use]
    pub fn new(store: ApprovalStore) -> Self {
        Self {
            store,
            audit: Vec::new(),
            next_id: 1,
        }
    }

    #[must_use]
    pub fn from_records(records: Vec<ApprovalRequestRecord>) -> Self {
        let store = ApprovalStore::default();
        let mut next_id = 1;
        for record in records {
            if let Some(suffix) = record.id.strip_prefix("approval.")
                && let Ok(id) = suffix.parse::<u64>()
            {
                next_id = next_id.max(id + 1);
            }
            store.insert(record);
        }
        Self {
            store,
            audit: Vec::new(),
            next_id,
        }
    }

    pub fn request(
        &mut self,
        session_id: impl Into<String>,
        action: impl Into<String>,
    ) -> ApprovalRequestRecord {
        let action = action.into();
        self.request_operation(session_id, action.clone(), action)
    }

    pub fn request_operation(
        &mut self,
        session_id: impl Into<String>,
        action: impl Into<String>,
        operation_id: impl Into<String>,
    ) -> ApprovalRequestRecord {
        self.request_operation_with_payload_hash(session_id, action, operation_id, None::<String>)
    }

    pub fn request_operation_with_payload_hash(
        &mut self,
        session_id: impl Into<String>,
        action: impl Into<String>,
        operation_id: impl Into<String>,
        payload_hash: Option<impl Into<String>>,
    ) -> ApprovalRequestRecord {
        let record = ApprovalRequestRecord {
            id: format!("approval.{}", self.next_id),
            session_id: session_id.into(),
            action: action.into(),
            operation_id: operation_id.into(),
            payload_hash: payload_hash.map(Into::into),
            consumed: false,
            state: ApprovalState::Pending,
        };
        self.next_id += 1;
        self.store.insert(record.clone());
        self.audit_event(record.id(), "requested");
        record
    }

    pub fn request_terminal_command(
        &mut self,
        session_id: impl Into<String>,
        request: &TerminalCommandRequest,
    ) -> TerminalCommandApproval {
        let record = self.request(session_id, "terminal.command");
        terminal_approval(&record, request)
    }

    pub fn resolve(
        &mut self,
        approval_id: &str,
        resolution: ApprovalResolution,
    ) -> Result<ApprovalRequestRecord, &'static str> {
        let Some(current) = self.store.get(approval_id) else {
            return Err("approval_missing");
        };
        let state = match resolution {
            ApprovalResolution::Approve => ApprovalState::Approved,
            ApprovalResolution::Deny => ApprovalState::Denied,
        };
        if current.state == state {
            return Ok(current);
        }
        if current.state == ApprovalState::Expired {
            return Err("approval_expired");
        }
        if current.state != ApprovalState::Pending {
            return Err("approval_already_resolved");
        }
        let updated = self
            .store
            .update_state(approval_id, state)
            .expect("approval should still exist");
        self.audit_event(approval_id, "resolved");
        Ok(updated)
    }

    pub fn expire(&mut self, approval_id: &str) {
        if !self
            .store
            .get(approval_id)
            .is_some_and(|record| record.state == ApprovalState::Pending)
        {
            return;
        }
        if self
            .store
            .update_state(approval_id, ApprovalState::Expired)
            .is_some()
        {
            self.audit_event(approval_id, "expired");
        }
    }

    pub fn resume_waiting_session(&self, session: &mut SessionWaitState) {
        let SessionWaitState::Blocked { approval_id } = session else {
            return;
        };
        if self
            .store
            .get(approval_id)
            .is_some_and(|record| record.state == ApprovalState::Approved)
        {
            *session = SessionWaitState::Resumed;
        }
    }

    #[must_use]
    pub fn get(&self, approval_id: &str) -> Option<ApprovalRequestRecord> {
        self.store.get(approval_id)
    }

    #[must_use]
    pub fn is_approved_for(&self, approval_id: &str, action: &str, operation_id: &str) -> bool {
        self.store.get(approval_id).is_some_and(|record| {
            record.state == ApprovalState::Approved
                && !record.consumed
                && record.action == action
                && record.operation_id == operation_id
        })
    }

    #[must_use]
    pub fn matches_payload(
        &self,
        approval_id: &str,
        action: &str,
        operation_id: &str,
        payload_hash: Option<&str>,
    ) -> bool {
        self.store.get(approval_id).is_some_and(|record| {
            record.action == action
                && record.operation_id == operation_id
                && record.payload_hash.as_deref() == payload_hash
        })
    }

    #[must_use]
    pub fn list(&self) -> Vec<ApprovalRequestRecord> {
        self.store.list()
    }

    pub fn terminal_command_decision(
        &self,
        approval_id: &str,
        request: &TerminalCommandRequest,
    ) -> Option<TerminalCommandApproval> {
        let record = self.store.get(approval_id)?;
        Some(terminal_approval(&record, request))
    }

    #[must_use]
    pub fn audit_count(&self) -> usize {
        self.audit.len()
    }

    pub(crate) fn audit_event(&mut self, approval_id: &str, event: &str) {
        self.audit.push(ApprovalAuditRecord {
            approval_id: approval_id.to_string(),
            event: event.to_string(),
        });
    }
}
