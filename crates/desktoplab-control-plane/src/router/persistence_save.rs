use desktoplab_storage::{ProductizationRecordKind, StorageError};

use super::LocalApiRouter;
use super::helpers::workspace_json;
use super::persistence_payloads;

impl LocalApiRouter {
    pub(crate) fn record_state_journal_result(&mut self, result: Result<(), StorageError>) {
        if let Err(error) = result
            && self.state_journal_fault.is_none()
        {
            self.state_journal_fault = Some(error.to_string());
        }
    }

    pub(crate) fn state_journal_failure(&self) -> Option<String> {
        self.state_journal_fault
            .clone()
            .or_else(|| self.sessions.persistence_fault())
    }

    pub(crate) fn persist_current_workspace(&mut self) {
        let (Some(storage), Some(workspace)) = (&self.storage, &self.workspace) else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::CurrentWorkspace,
            "current",
            workspace_json(workspace),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_event_outbox(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::BackendEventOutbox,
            "local",
            self.events.to_json(),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_workspace_registry(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::WorkspaceRegistry,
            "local",
            persistence_payloads::workspace_registry_payload(
                &self.workspaces,
                &self.archived_workspace_ids,
                &self.archived_session_ids,
            ),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_setup_state(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::SetupState,
            "local",
            self.setup.to_json(),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_readiness_state(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::BackendReadiness,
            "local",
            self.readiness.to_json(),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_setup_pipeline(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::SetupPipeline,
            "local",
            self.setup_pipeline.to_json(),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_provider_account(&mut self, provider_id: &str) {
        let (Some(storage), Some(account)) =
            (&self.storage, self.provider_accounts.get(provider_id))
        else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::ProviderAccount,
            provider_id,
            account.to_json(),
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_workspace_memory(&mut self, workspace_id: &str) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::WorkspaceMemory,
            workspace_id,
            persistence_payloads::workspace_memory_payload(
                self.workspace_memories
                    .get(workspace_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
        );
        self.record_state_journal_result(result);
    }
}
