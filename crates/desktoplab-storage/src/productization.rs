#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductizationRecordKind {
    ProviderAccount,
    RuntimeInventory,
    ModelInventory,
    RouteHistory,
    CurrentWorkspace,
    WorkspaceRegistry,
    AgentSession,
    WorkspaceMemory,
    PluginTrust,
    WorktreeSession,
    SubagentSession,
    RegistryCache,
    SetupState,
    BackendReadiness,
    SetupPipeline,
    RuntimeJob,
    ModelJob,
    ApprovalRecord,
    AgentPendingAction,
    AgentActiveSession,
    WorkflowRun,
    OperatorRunCard,
    ExtensionRegistry,
    ReviewWorkUnit,
    BackendEventOutbox,
    AgentContextCompaction,
    McpServerRegistry,
    Unknown,
}

impl ProductizationRecordKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderAccount => "provider_account",
            Self::RuntimeInventory => "runtime_inventory",
            Self::ModelInventory => "model_inventory",
            Self::RouteHistory => "route_history",
            Self::CurrentWorkspace => "current_workspace",
            Self::WorkspaceRegistry => "workspace_registry",
            Self::AgentSession => "agent_session",
            Self::WorkspaceMemory => "workspace_memory",
            Self::PluginTrust => "plugin_trust",
            Self::WorktreeSession => "worktree_session",
            Self::SubagentSession => "subagent_session",
            Self::RegistryCache => "registry_cache",
            Self::SetupState => "setup_state",
            Self::BackendReadiness => "backend_readiness",
            Self::SetupPipeline => "setup_pipeline",
            Self::RuntimeJob => "runtime_job",
            Self::ModelJob => "model_job",
            Self::ApprovalRecord => "approval_record",
            Self::AgentPendingAction => "agent_pending_action",
            Self::AgentActiveSession => "agent_active_session",
            Self::WorkflowRun => "workflow_run",
            Self::OperatorRunCard => "operator_run_card",
            Self::ExtensionRegistry => "extension_registry",
            Self::ReviewWorkUnit => "review_work_unit",
            Self::BackendEventOutbox => "backend_event_outbox",
            Self::AgentContextCompaction => "agent_context_compaction",
            Self::McpServerRegistry => "mcp_server_registry",
            Self::Unknown => "unknown",
        }
    }

    #[must_use]
    pub fn from_storage(value: &str) -> Self {
        match value {
            "provider_account" => Self::ProviderAccount,
            "runtime_inventory" => Self::RuntimeInventory,
            "model_inventory" => Self::ModelInventory,
            "route_history" => Self::RouteHistory,
            "current_workspace" => Self::CurrentWorkspace,
            "workspace_registry" => Self::WorkspaceRegistry,
            "agent_session" => Self::AgentSession,
            "workspace_memory" => Self::WorkspaceMemory,
            "plugin_trust" => Self::PluginTrust,
            "worktree_session" => Self::WorktreeSession,
            "subagent_session" => Self::SubagentSession,
            "registry_cache" => Self::RegistryCache,
            "setup_state" => Self::SetupState,
            "backend_readiness" => Self::BackendReadiness,
            "setup_pipeline" => Self::SetupPipeline,
            "runtime_job" => Self::RuntimeJob,
            "model_job" => Self::ModelJob,
            "approval_record" => Self::ApprovalRecord,
            "agent_pending_action" => Self::AgentPendingAction,
            "agent_active_session" => Self::AgentActiveSession,
            "workflow_run" => Self::WorkflowRun,
            "operator_run_card" => Self::OperatorRunCard,
            "extension_registry" => Self::ExtensionRegistry,
            "review_work_unit" => Self::ReviewWorkUnit,
            "backend_event_outbox" => Self::BackendEventOutbox,
            "agent_context_compaction" => Self::AgentContextCompaction,
            "mcp_server_registry" => Self::McpServerRegistry,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductizationStateRecord {
    kind: ProductizationRecordKind,
    subject_id: String,
    payload: String,
    updated_at: String,
}

impl ProductizationStateRecord {
    #[must_use]
    pub fn new(
        kind: ProductizationRecordKind,
        subject_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject_id: subject_id.into(),
            payload: payload.into(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    #[must_use]
    pub fn from_storage(
        kind: ProductizationRecordKind,
        subject_id: impl Into<String>,
        payload: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject_id: subject_id.into(),
            payload: payload.into(),
            updated_at: updated_at.into(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> ProductizationRecordKind {
        self.kind
    }

    #[must_use]
    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }
}
