use desktoplab_backend_services::{
    ApprovalService, ApprovalStore, AuditLogService, AuditStore, BackendEventStreamService,
    JobService, JobServiceStore, SessionService, SessionServiceStore,
};
use desktoplab_domain::ApprovalMode;
use desktoplab_storage::SqliteStore;
use desktoplab_vault::FakeVault;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use crate::BackendReadinessState;
use crate::lifecycle::StabilityClock;
use crate::provider_accounts::ProviderAccountRecord;
use crate::provider_bridge_routes::OpenAiCodexPairingRecord;
use crate::setup_pipeline::SetupPipeline;
use crate::setup_state::SetupState;
use std::path::{Path, PathBuf};

mod agent_attachments;
#[cfg(debug_assertions)]
mod agent_backend_recovery;
mod agent_compaction;
mod agent_context;
#[cfg(debug_assertions)]
mod agent_continuation;
mod agent_execution_binding;
mod agent_iterative;
mod agent_iterative_resume;
mod agent_memory;
mod agent_model_constrained;
mod agent_model_execution;
mod agent_model_jobs;
mod agent_model_local;
mod agent_model_tools;
mod agent_observation_display;
mod agent_pending;
mod agent_plan_tools;
#[cfg(debug_assertions)]
mod agent_refactor;
mod agent_sessions;
mod agent_subagent_tools;
pub(crate) mod agent_transcript;
mod approval_modes;
mod approvals;
mod diagnostics;
mod diagnostics_doctor;
mod diagnostics_export;
mod dispatch;
mod git_fingerprint;
mod helpers;
mod high_end_runtime;
mod mcp;
mod mcp_persistence;
mod migration_status;
mod model_setup;
mod payload_hash;
mod persistence;
mod persistence_agent;
mod persistence_iterative;
mod persistence_jobs;
mod persistence_load;
mod persistence_payloads;
mod persistence_route_selection;
mod persistence_save;
mod persistence_settings;
mod plugin_routes;
mod route_selection;
mod runtime_setup;
mod security_audit;
mod setup_recovery;
mod setup_runtime_model;
mod stability_diagnostics;
mod subagent_change_review;
mod subagents;
#[cfg(debug_assertions)]
mod test_controls;
#[cfg(debug_assertions)]
mod test_fixtures;
mod workspace_agent;
mod workspace_identity;
mod workspace_surfaces;
mod worktree_bindings;

pub struct LocalApiRouter {
    pub(crate) workspace: Option<WorkspaceRecord>,
    pub(crate) workspaces: BTreeMap<String, WorkspaceRecord>,
    pub(crate) archived_workspace_ids: BTreeSet<String>,
    pub(crate) archived_session_ids: BTreeSet<String>,
    pub(crate) sessions: SessionService,
    pub(crate) approvals: ApprovalService,
    pub(crate) jobs: JobService,
    pub(crate) events: BackendEventStreamService,
    pub(crate) agent_pending_actions: BTreeMap<String, agent_pending::PendingAgentAction>,
    pub(crate) agent_completed_actions: BTreeMap<String, agent_sessions::PendingExecutionOutcome>,
    pub(crate) agent_iterative_states:
        BTreeMap<String, desktoplab_agent_engine::IterativeLoopState>,
    pub(crate) agent_iterative_event_offsets: BTreeMap<String, usize>,
    pub(crate) agent_iterative_prompts: BTreeMap<String, String>,
    pub(crate) agent_execution_bindings:
        BTreeMap<String, agent_execution_binding::AgentExecutionBinding>,
    pub(crate) agent_model_inflight: BTreeSet<String>,
    #[cfg(debug_assertions)]
    pub(crate) agent_model_delay_for_test: Option<std::time::Duration>,
    pub(crate) agent_streaming_sessions: BTreeSet<String>,
    pub(crate) agent_cancellation_tokens:
        BTreeMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) agent_process_registry: desktoplab_tool_gateway::SharedProcessRegistry,
    pub(crate) mcp_runtime: desktoplab_tool_gateway::SharedMcpRuntime,
    pub(crate) mcp_servers: BTreeMap<String, mcp_persistence::McpServerRegistration>,
    pub(crate) mcp_reconnect_failures: BTreeMap<String, String>,
    pub(crate) agent_context_compactions:
        BTreeMap<String, agent_compaction::AgentContextCompaction>,
    pub(crate) workspace_memories: BTreeMap<String, Vec<agent_memory::WorkspaceMemoryRecord>>,
    #[cfg(debug_assertions)]
    pub(crate) agent_last_file_path_by_workspace: BTreeMap<String, String>,
    pub(crate) agent_active_session_by_workspace: BTreeMap<String, String>,
    pub(crate) worktree_bindings: BTreeMap<String, worktree_bindings::WorktreeBinding>,
    pub(crate) subagents: BTreeMap<String, subagents::SubagentRecord>,
    pub(crate) plugin_trust: BTreeMap<String, plugin_routes::PluginTrustRecord>,
    pub(crate) audit: AuditLogService,
    pub(crate) storage: Option<SqliteStore>,
    pub(crate) state_journal_fault: Option<String>,
    pub(crate) setup: SetupState,
    pub(crate) setup_pipeline: SetupPipeline,
    pub(crate) readiness: BackendReadinessState,
    pub(crate) default_approval_mode: ApprovalMode,
    pub(crate) session_approval_mode: ApprovalMode,
    pub(crate) selected_route_id: String,
    pub(crate) stability: StabilityClock,
    pub(crate) provider_accounts: BTreeMap<String, ProviderAccountRecord>,
    pub(crate) openai_codex_pairings: BTreeMap<String, OpenAiCodexPairingRecord>,
    pub(crate) openai_codex_device_authorization_for_test: Option<OpenAiCodexDeviceFixture>,
    pub(crate) openai_codex_bridge_dir: Option<PathBuf>,
    pub(crate) openai_codex_native_vault_for_test: Option<FakeVault>,
    pub(crate) runtime_verification_for_test: Option<RuntimeVerificationFixture>,
    pub(crate) local_model_inventory_for_test: Option<Vec<String>>,
    pub(crate) ollama_model_capabilities: desktoplab_backends::OllamaModelCapabilityResolver,
    pub(crate) ollama_tool_protocol_canary: desktoplab_backends::OllamaToolProtocolCanary,
    pub(crate) host_memory_gb: u32,
    pub(crate) host_memory_gb_for_test: Option<u32>,
    pub(crate) model_download_execution: ModelDownloadExecutionMode,
    pub(crate) agent_backend_execution: AgentBackendExecutionMode,
    pub(crate) legacy_agent_test_harness_enabled: bool,
    pub(crate) managed_runtime_marker_path: Option<PathBuf>,
    pub(crate) managed_runtime_owner_id: Option<String>,
    pub(crate) high_end_runtime: Option<desktoplab_runtime::HighEndRuntimeLifecycle>,
    #[cfg(debug_assertions)]
    pub(crate) test_controls_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelDownloadExecutionMode {
    Execute,
    #[cfg(debug_assertions)]
    PlanOnlyForTest,
    #[cfg(debug_assertions)]
    CompleteForTest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AgentBackendExecutionMode {
    Execute,
    #[cfg(debug_assertions)]
    DeterministicForTest(String),
    #[cfg(debug_assertions)]
    DeterministicSequenceForTest(Vec<String>),
    #[cfg(debug_assertions)]
    NativeIterativeSequenceForTest(Vec<String>),
    #[cfg(debug_assertions)]
    FailForTest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenAiCodexDeviceFixture {
    pub(crate) device_auth_id: String,
    pub(crate) user_code: String,
    pub(crate) authorization_code: String,
    pub(crate) code_verifier: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeVerificationFixture {
    pub(crate) verified: bool,
    pub(crate) evidence: String,
    pub(crate) blocked_reason: String,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceRecord {
    pub(crate) workspace_id: String,
    pub(crate) display_name: String,
    pub(crate) root_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiRouteResponse {
    status: &'static str,
    body: String,
}

impl ApiRouteResponse {
    pub(crate) fn ok(body: Value) -> Self {
        Self {
            status: "200 OK",
            body: body.to_string(),
        }
    }

    pub(crate) fn not_found() -> Self {
        Self {
            status: "404 Not Found",
            body: json!({"code":"NOT_FOUND","message":"route not found"}).to_string(),
        }
    }

    pub(crate) fn bad_request(body: Value) -> Self {
        Self {
            status: "400 Bad Request",
            body: body.to_string(),
        }
    }

    pub(crate) fn state_journal_failed(error: impl std::fmt::Display) -> Self {
        Self {
            status: "500 Internal Server Error",
            body: json!({
                "code":"STATE_JOURNAL_FAILED",
                "message":"DesktopLab could not durably record the state transition.",
                "detail":error.to_string()
            })
            .to_string(),
        }
    }

    pub fn status(&self) -> &'static str {
        self.status
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

impl Default for LocalApiRouter {
    fn default() -> Self {
        Self {
            workspace: None,
            workspaces: BTreeMap::new(),
            archived_workspace_ids: BTreeSet::new(),
            archived_session_ids: BTreeSet::new(),
            sessions: SessionService::new(SessionServiceStore::default()),
            approvals: ApprovalService::new(ApprovalStore::default()),
            jobs: JobService::new(JobServiceStore::default()),
            events: BackendEventStreamService::default(),
            agent_pending_actions: BTreeMap::new(),
            agent_completed_actions: BTreeMap::new(),
            agent_iterative_states: BTreeMap::new(),
            agent_iterative_event_offsets: BTreeMap::new(),
            agent_iterative_prompts: BTreeMap::new(),
            agent_execution_bindings: BTreeMap::new(),
            agent_model_inflight: BTreeSet::new(),
            #[cfg(debug_assertions)]
            agent_model_delay_for_test: None,
            agent_streaming_sessions: BTreeSet::new(),
            agent_cancellation_tokens: BTreeMap::new(),
            agent_process_registry: desktoplab_tool_gateway::SharedProcessRegistry::default(),
            mcp_runtime: desktoplab_tool_gateway::SharedMcpRuntime::default(),
            mcp_servers: BTreeMap::new(),
            mcp_reconnect_failures: BTreeMap::new(),
            agent_context_compactions: BTreeMap::new(),
            workspace_memories: BTreeMap::new(),
            #[cfg(debug_assertions)]
            agent_last_file_path_by_workspace: BTreeMap::new(),
            agent_active_session_by_workspace: BTreeMap::new(),
            worktree_bindings: BTreeMap::new(),
            subagents: BTreeMap::new(),
            plugin_trust: BTreeMap::new(),
            audit: AuditLogService::new(AuditStore::default()),
            storage: None,
            state_journal_fault: None,
            setup: SetupState::default(),
            setup_pipeline: SetupPipeline::default(),
            readiness: BackendReadinessState::default(),
            default_approval_mode: ApprovalMode::default(),
            session_approval_mode: ApprovalMode::default(),
            selected_route_id: crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string(),
            stability: StabilityClock::default(),
            provider_accounts: BTreeMap::new(),
            openai_codex_pairings: BTreeMap::new(),
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
            model_download_execution: ModelDownloadExecutionMode::Execute,
            agent_backend_execution: AgentBackendExecutionMode::Execute,
            legacy_agent_test_harness_enabled: false,
            managed_runtime_marker_path: None,
            managed_runtime_owner_id: None,
            high_end_runtime: None,
            #[cfg(debug_assertions)]
            test_controls_enabled: false,
        }
    }
}

impl LocalApiRouter {
    #[must_use]
    pub fn with_managed_runtime_ownership(
        mut self,
        path: impl AsRef<Path>,
        owner_id: impl Into<String>,
    ) -> Self {
        self.managed_runtime_marker_path = Some(path.as_ref().to_path_buf());
        self.managed_runtime_owner_id = Some(owner_id.into());
        self
    }

    #[must_use]
    pub fn with_openai_codex_bridge_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.openai_codex_bridge_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub(crate) fn owns_managed_ollama_runtime(&self) -> bool {
        let (Some(path), Some(owner_id)) = (
            &self.managed_runtime_marker_path,
            &self.managed_runtime_owner_id,
        ) else {
            return false;
        };
        std::fs::read_to_string(path).is_ok_and(|marker| marker.trim() == owner_id)
    }

    #[must_use]
    pub fn should_shutdown_ollama_on_desktop_exit(&self) -> bool {
        self.owns_managed_ollama_runtime()
    }
}
