use desktoplab_backend_services::{
    ApprovalService, AuditLogService, AuditStore, JobService, SessionService, SessionServiceStore,
};
use desktoplab_storage::{SqliteStore, StorageError};
use std::collections::BTreeMap;
use std::path::Path;

use super::LocalApiRouter;
use super::mcp_persistence::load_mcp_servers;
use super::persistence_iterative::load_agent_iterative_journal;
use super::persistence_jobs::load_background_job_store;
use super::persistence_load::{
    load_agent_active_sessions, load_agent_context_compactions, load_agent_pending_actions,
    load_approval_records, load_backend_event_outbox, load_backend_readiness_state,
    load_current_workspace, load_default_approval_mode, load_high_end_runtime,
    load_provider_accounts, load_selected_route_id, load_setup_pipeline, load_setup_state,
    load_workspace_memories, load_workspace_registry,
};
use super::plugin_routes::load_plugin_trust;
use super::subagents::load_subagents;
use super::worktree_bindings::load_worktree_bindings;

impl LocalApiRouter {
    pub fn with_storage(storage: SqliteStore) -> Result<Self, StorageError> {
        Self::with_storage_and_sessions(storage, SessionServiceStore::default())
    }

    pub fn with_storage_path(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::with_storage_path_and_host_recovery(path, true)
    }

    pub fn with_storage_path_without_host_recovery_for_test(
        path: impl AsRef<Path>,
    ) -> Result<Self, StorageError> {
        Self::with_storage_path_and_host_recovery(path, false)
    }

    fn with_storage_path_and_host_recovery(
        path: impl AsRef<Path>,
        recover_existing_host_setup: bool,
    ) -> Result<Self, StorageError> {
        let storage = SqliteStore::open(path.as_ref())?;
        storage.apply_migrations()?;
        let session_storage = SqliteStore::open(path)?;
        session_storage.apply_migrations()?;
        let session_store = SessionServiceStore::with_storage(session_storage)?;
        Self::with_storage_and_sessions_and_host_recovery(
            storage,
            session_store,
            recover_existing_host_setup,
        )
    }

    pub(crate) fn with_storage_and_sessions(
        storage: SqliteStore,
        session_store: SessionServiceStore,
    ) -> Result<Self, StorageError> {
        Self::with_storage_and_sessions_and_host_recovery(storage, session_store, true)
    }

    fn with_storage_and_sessions_and_host_recovery(
        storage: SqliteStore,
        session_store: SessionServiceStore,
        recover_existing_host_setup: bool,
    ) -> Result<Self, StorageError> {
        let current_workspace = load_current_workspace(&storage)?;
        let events = load_backend_event_outbox(&storage)?;
        let agent_context_compactions = load_agent_context_compactions(&storage)?;
        let approval_records = load_approval_records(&storage)?;
        let (mut workspaces, archived_workspace_ids, archived_session_ids) =
            load_workspace_registry(&storage)?;
        if let Some(workspace) = &current_workspace {
            workspaces.insert(workspace.workspace_id.clone(), workspace.clone());
        }
        let high_end_runtime = load_high_end_runtime(&storage)?;
        let mcp_servers = load_mcp_servers(&storage)?;
        let (
            agent_iterative_states,
            agent_iterative_prompts,
            agent_iterative_event_offsets,
            agent_streaming_sessions,
            agent_execution_bindings,
        ) = load_agent_iterative_journal(&storage)?;
        let mut router = Self {
            workspace: current_workspace,
            workspaces,
            archived_workspace_ids,
            archived_session_ids,
            sessions: SessionService::new(session_store),
            approvals: ApprovalService::from_records(approval_records),
            jobs: JobService::new(load_background_job_store(&storage)?),
            events,
            agent_pending_actions: load_agent_pending_actions(&storage)?,
            agent_completed_actions: BTreeMap::new(),
            agent_iterative_states,
            agent_iterative_event_offsets,
            agent_iterative_prompts,
            agent_execution_bindings,
            agent_model_inflight: std::collections::BTreeSet::new(),
            #[cfg(debug_assertions)]
            agent_model_delay_for_test: None,
            agent_streaming_sessions,
            agent_cancellation_tokens: BTreeMap::new(),
            agent_process_registry: desktoplab_tool_gateway::SharedProcessRegistry::default(),
            mcp_runtime: desktoplab_tool_gateway::SharedMcpRuntime::default(),
            mcp_servers,
            mcp_reconnect_failures: BTreeMap::new(),
            agent_context_compactions,
            workspace_memories: load_workspace_memories(&storage)?,
            #[cfg(debug_assertions)]
            agent_last_file_path_by_workspace: BTreeMap::new(),
            agent_active_session_by_workspace: load_agent_active_sessions(&storage)?,
            worktree_bindings: load_worktree_bindings(&storage)?,
            subagents: load_subagents(&storage)?,
            plugin_trust: load_plugin_trust(&storage)?,
            audit: AuditLogService::new(AuditStore::default()),
            setup: load_setup_state(&storage)?,
            setup_pipeline: load_setup_pipeline(&storage)?,
            readiness: load_backend_readiness_state(&storage)?,
            default_approval_mode: load_default_approval_mode(&storage)?,
            session_approval_mode: load_default_approval_mode(&storage)?,
            selected_route_id: load_selected_route_id(&storage)?,
            stability: crate::lifecycle::StabilityClock::default(),
            provider_accounts: load_provider_accounts(&storage)?,
            openai_codex_pairings: std::collections::BTreeMap::new(),
            openai_codex_device_authorization_for_test: None,
            openai_codex_bridge_dir: None,
            openai_codex_native_vault_for_test: None,
            runtime_verification_for_test: None,
            local_model_inventory_for_test: None,
            ollama_model_capabilities: desktoplab_backends::OllamaModelCapabilityResolver::default(
            ),
            ollama_tool_protocol_canary: desktoplab_backends::OllamaToolProtocolCanary::default(),
            host_memory_gb: crate::model_routes::effective_memory_gb(),
            host_memory_gb_for_test: None,
            model_download_execution: super::ModelDownloadExecutionMode::Execute,
            agent_backend_execution: super::AgentBackendExecutionMode::Execute,
            legacy_agent_test_harness_enabled: false,
            managed_runtime_marker_path: None,
            managed_runtime_owner_id: None,
            high_end_runtime,
            #[cfg(debug_assertions)]
            test_controls_enabled: false,
            storage: Some(storage),
            state_journal_fault: None,
        };
        router.recover_interrupted_agent_jobs();
        router.reconnect_mcp_servers();
        if router.workspace.is_some() {
            router.persist_current_workspace();
        }
        router.reconcile_agent_model_catalog();
        router.reconcile_restored_high_end_route();
        router.recover_stale_runtime_install();
        if recover_existing_host_setup {
            router.recover_existing_host_setup();
        }
        router.persist_event_outbox();
        if let Some(error) = router.state_journal_failure() {
            return Err(StorageError::Sqlite(error));
        }
        Ok(router)
    }
}
