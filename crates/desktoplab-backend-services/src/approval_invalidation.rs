use crate::{ApprovalService, ApprovalState, ApprovalStore};

impl ApprovalStore {
    fn invalidate_unconsumed_for_session(&self, session_id: &str) -> Vec<String> {
        let mut records = self
            .records
            .lock()
            .expect("approval store lock should not be poisoned");
        records
            .iter_mut()
            .filter(|record| {
                record.session_id == session_id
                    && !record.consumed
                    && matches!(
                        record.state,
                        ApprovalState::Pending | ApprovalState::Approved
                    )
            })
            .map(|record| {
                record.state = ApprovalState::Expired;
                record.id.clone()
            })
            .collect()
    }
}

impl ApprovalService {
    pub fn invalidate_unconsumed_for_session(&mut self, session_id: &str) {
        for approval_id in self.store.invalidate_unconsumed_for_session(session_id) {
            self.audit_event(&approval_id, "expired");
        }
    }
}
