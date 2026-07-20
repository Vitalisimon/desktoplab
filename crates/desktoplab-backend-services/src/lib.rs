#![forbid(unsafe_code)]

mod approval;
mod approval_consumption;
mod approval_invalidation;
mod approval_terminal;
mod audit;
mod composition;
mod diagnostics;
mod diagnostics_repair;
mod event_stream;
mod jobs;
mod performance;
mod plugin_host;
mod plugin_inspector;
mod plugin_productization;
mod provider_credentials;
mod provider_probes;
mod provider_productization;
mod resource_guards;
mod review_work_units;
mod routing;
mod run_cards;
mod session_job_service;
mod session_queue_service;
mod session_recovery;
mod session_storage;
mod session_trace;
mod session_trace_metadata;
mod session_turns;
mod sessions;
mod setup_selection;
mod setup_wizard;
mod workflow;
mod workflow_validation;

pub use approval::{
    ApprovalRequestRecord, ApprovalResolution, ApprovalService, ApprovalState, ApprovalStore,
    SessionWaitState,
};
pub use approval_terminal::TerminalCommandApproval;
pub use audit::{
    AuditAction, AuditDecisionSummary, AuditLogService, AuditQuery, AuditRecord, AuditStore,
    LocalAuditTransparencySnapshot,
};
pub use composition::{
    BackendServices, ServiceDescriptor, ServiceHealth, ServiceKind, ServiceReadiness,
    ServiceRequirement,
};
pub use diagnostics::{
    BackendDiagnosticsService, DiagnosticServiceFamily, DiagnosticServiceState,
    DiagnosticsSnapshot, ModelDownloadDiagnosticFailure, PackagingDiagnostics,
};
pub use diagnostics_repair::{
    DiagnosticsRepairPlan, DiagnosticsRepairPlanner, RepairAction, RepairActionFamily,
    RepairActionMode,
};
pub use event_stream::{
    BackendEventFrame, BackendEventScope, BackendEventStreamService, EventReplayRequest,
    EventReplayResponse,
};
pub use jobs::{
    JobId, JobRetryClass, JobService, JobServiceStore, JobSnapshot, JobState, SseReplay,
};
pub use performance::{DiagnosticsBundleGuard, ProductizationPerformanceGate};
pub use plugin_host::{
    LoadedPlugin, PluginCompatibility, PluginHost, PluginManifest, PluginRouteDecision,
    PluginRouteStatus, PluginTrustState,
};
pub use plugin_inspector::{
    PluginCompatibilityFinding, PluginCompatibilityInspector, PluginCompatibilityReport,
    PluginFindingSeverity,
};
pub use plugin_productization::{
    PluginAuthorization, PluginContractHook, PluginDistributionKind, PluginExecutionBoundary,
    PluginExecutionBoundaryKind, PluginHookKind, PluginInstallBoundary, PluginManifestLoader,
    PluginPermissionEngine, PluginPermissionKind, PluginProductManifest, PluginProductizationHost,
    PluginRuntimeState, PluginTrustAction,
};
pub use provider_credentials::{
    ProviderAccount, ProviderCredentialError, ProviderCredentialService, ProviderCredentialStore,
    ProviderReadiness,
};
pub use provider_probes::{
    ProviderProbeConfidence, ProviderProbeDefinition, ProviderProbeError, ProviderProbeExecution,
    ProviderProbeExecutor, ProviderProbeInitiation, ProviderProbePermission, ProviderProbeReport,
    ProviderProbeRequest, ProviderProbeService, ProviderProbeSource, ProviderProbeState,
};
pub use provider_productization::{
    ProviderCandidate, ProviderConnectivityDiagnostic, ProviderConnectivityInput,
    ProviderConnectivityState, ProviderEndpointClass, ProviderEndpointError,
    ProviderEndpointMetadata, ProviderManifestTrust, ProviderPluginError, ProviderPluginManifest,
    ProviderProductizationCatalog, ProviderReadinessReport, ProviderReadinessStatus,
    ProviderRouteDecision, ProviderRoutePlanner, ProviderRoutePreference, ProviderSpec,
};
pub use resource_guards::{
    ApiPayloadGuard, DownloadDiskGuard, WorkspaceScanDecision, WorkspaceScanGuard,
};
pub use review_work_units::{
    DeliveryKind, PatchAttempt, ReviewFinding, ReviewWorkUnit, ReviewWorkUnitError,
    ReviewWorkUnitService, VerificationRecord,
};
pub use routing::{
    BackendRouteCandidate, BackendRouteDecision, BackendRouteService, BackendRouteStatus,
    FallbackVisibility, RouteApiPolicy, RouteApiRequest,
};
pub use run_cards::{
    OperatorRunCard, RunCardError, RunCardEvent, RunCardService, RunCardState, TakeoverRequest,
};
pub use session_recovery::{SessionContinuation, SessionRecoverySnapshot};
pub use session_trace::{SessionTraceEnvelope, SessionTraceEvent};
pub use session_turns::SessionTurnSnapshot;
pub use sessions::{SessionService, SessionServiceStore};
pub use setup_selection::{SetupAcceptance, SetupPlanSelection};
pub use setup_wizard::{
    CatalogChannel, CatalogEntryKind, CatalogRefreshRequestResult, CatalogRefreshRequestState,
    CatalogRefreshStatus, SetupCatalogEntry, SetupPlanPreview, SetupRecommendation,
    SetupRecommendationRole, SetupWizardApiService, SetupWizardPolicy, SetupWizardRegistryState,
};
pub use workflow::{
    WorkflowDefinition, WorkflowError, WorkflowExecutor, WorkflowGraph, WorkflowNode,
    WorkflowNodeKind, WorkflowProgress, WorkflowService, WorkflowStatus, WorkflowStepOutcome,
};
