use crate::{ApprovalRequestRecord, ApprovalService, ApprovalState, ApprovalStore};

impl ApprovalStore {
    fn consume_matching(
        &self,
        approval_id: &str,
        session_id: &str,
        action: &str,
        operation_id: &str,
        payload_hash: Option<&str>,
    ) -> Option<ApprovalRequestRecord> {
        let mut records = self
            .records
            .lock()
            .expect("approval store lock should not be poisoned");
        let record = records.iter_mut().find(|record| record.id == approval_id)?;
        if record.state != ApprovalState::Approved
            || record.consumed
            || record.session_id != session_id
            || record.action != action
            || record.operation_id != operation_id
            || record.payload_hash.as_deref() != payload_hash
        {
            return None;
        }
        record.consumed = true;
        Some(record.clone())
    }
}

impl ApprovalService {
    pub fn consume_approved_for_payload(
        &mut self,
        approval_id: &str,
        session_id: &str,
        action: &str,
        operation_id: &str,
        payload_hash: Option<&str>,
    ) -> bool {
        if self
            .store
            .consume_matching(approval_id, session_id, action, operation_id, payload_hash)
            .is_some()
        {
            self.audit_event(approval_id, "consumed");
            return true;
        }
        false
    }
}
