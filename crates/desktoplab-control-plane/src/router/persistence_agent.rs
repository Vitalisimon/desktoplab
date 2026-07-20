use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, StorageError};

use super::LocalApiRouter;
use super::persistence_payloads;
use super::plugin_routes::trust_payload;
use super::subagents::subagent_payload;
use super::worktree_bindings::bindings_payload;

impl LocalApiRouter {
    pub(crate) fn persist_agent_approval_journal(&self) -> Result<(), StorageError> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        storage.put_productization_states(&[
            ProductizationStateRecord::new(
                ProductizationRecordKind::ApprovalRecord,
                "local",
                persistence_payloads::approval_records_payload(&self.approvals).to_string(),
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::AgentPendingAction,
                "local",
                persistence_payloads::pending_actions_payload(&self.agent_pending_actions)
                    .to_string(),
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::AgentSession,
                "iterative",
                persistence_payloads::iterative_states_payload(
                    &self.agent_iterative_states,
                    &self.agent_iterative_prompts,
                    &self.agent_iterative_event_offsets,
                    &self.agent_streaming_sessions,
                    &self.agent_execution_bindings,
                )
                .to_string(),
            ),
        ])
    }

    pub(crate) fn persist_agent_active_sessions(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::AgentActiveSession,
            "local",
            persistence_payloads::active_sessions_payload(&self.agent_active_session_by_workspace),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_worktree_bindings(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::WorktreeSession,
            "local",
            bindings_payload(&self.worktree_bindings),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_subagents(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::SubagentSession,
            "local",
            subagent_payload(&self.subagents),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_plugin_trust(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::PluginTrust,
            "local",
            trust_payload(&self.plugin_trust),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_agent_context_compactions(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::AgentContextCompaction,
            "local",
            persistence_payloads::context_compactions_payload(&self.agent_context_compactions),
        );
        self.record_state_journal_result(result);
    }
}
