#![forbid(unsafe_code)]

mod detection;
mod deterministic_process;
mod download;
mod execution;
mod execution_result;
mod high_end;
mod high_end_health;
mod high_end_http;
mod high_end_process;
mod install;
mod install_contract;
mod install_job;
mod installer_flow;
mod lm_studio;
mod lm_studio_discovery;
mod lm_studio_managed_execution;
mod lm_studio_managed_lifecycle;
mod lm_studio_managed_marker;
mod lm_studio_managed_plan;
mod lm_studio_managed_status;
mod macos_ollama_install;
mod manager;
mod mlx_lm;
mod mlx_lm_managed_execution;
mod mlx_lm_managed_lifecycle;
mod mlx_lm_managed_marker;
mod mlx_lm_managed_plan;
mod mlx_lm_managed_status;
mod ollama;
mod ollama_probe;
mod process;
mod process_spawn;
mod productization;
mod provenance;
mod runtime_lifecycle;
mod status;
mod supervisor;
mod windows_ollama_install;

pub use detection::{
    InMemoryRuntimeInventoryStore, LmStudioRuntimeDetector, OllamaRuntimeDetector,
    RuntimeDetectionEvent, RuntimeDetectionEventKind, RuntimeDetectionOutcome,
    RuntimeDetectionReport, RuntimeDetectionService, RuntimeDetector,
};
pub use deterministic_process::DeterministicProcessRunner;
pub use download::{
    RuntimeCachedInstallerArtifact, RuntimeDownloadFailure, RuntimeDownloadFailureKind,
    RuntimeDownloadRetryClass, RuntimeDownloadVerification,
};
pub use execution::{RuntimeInstallExecutionDesign, RuntimeInstallExecutor, RuntimeInstallPhase};
pub use execution_result::{RuntimeExecutionState, RuntimeInstallExecutionResult};
pub use high_end::{
    HighEndRuntimeContract, HighEndRuntimeFamily, RuntimeCapabilityState,
    RuntimeInferenceCapabilities, RuntimeLaunchSupport, RuntimeSessionOwnership,
    high_end_runtime_contracts,
};
pub use high_end_health::{
    HighEndRuntimeHealthEvidence, HighEndRuntimeHealthState, HighEndRuntimeLifecycle,
    HighEndRuntimeLifecycleError, HighEndRuntimeOwnership, HttpRuntimeEndpointProbe,
    RuntimeEndpointError, RuntimeEndpointHealthProbe, RuntimeEndpointSpec,
};
pub use high_end_process::{DesktopLabOwnedRuntimeProcess, HighEndLaunchSpec, HighEndProcessError};
pub use install::{InstallPlan, RuntimeInstallPlanner};
pub use install_contract::{
    InstallerSource, RuntimeInstallExecutionStrategy, RuntimeInstallManagement,
};
pub use install_job::{
    InstallHostCapacity, RuntimeInstallApproval, RuntimeInstallError, RuntimeInstallJob,
    RuntimeInstallRequest, RuntimeInstallStatus,
};
pub use lm_studio::{
    LmStudioEndpointDetection, LmStudioEndpointProbe, LmStudioGuidedSetupPlan, LmStudioHostAdapter,
    LmStudioLocalEndpointMetadata, LmStudioRuntime,
};
pub use lm_studio_discovery::{
    HttpLmStudioModelProbe, LmStudioDiscoveryState, LmStudioExistingDiscovery, LmStudioModelProbe,
    discover_existing_lm_studio, discover_system_lm_studio,
};
pub use lm_studio_managed_execution::execute_managed_lm_studio;
pub use lm_studio_managed_lifecycle::stop_managed_lm_studio;
pub use lm_studio_managed_plan::{
    LmStudioManagedArtifact, LmStudioManagedPhase, LmStudioManagedPlan, LmStudioManagedPlanError,
};
pub use lm_studio_managed_status::{
    LmStudioManagedState, LmStudioManagedStatus, inspect_managed_lm_studio,
};
pub use manager::{RuntimeCommand, RuntimeManager};
pub use mlx_lm::{
    MlxLmEndpointDetection, MlxLmEndpointProbe, MlxLmInstallPlanError, MlxLmLocalEndpointMetadata,
    MlxLmRuntime,
};
pub use mlx_lm_managed_execution::execute_managed_mlx_lm;
pub use mlx_lm_managed_lifecycle::stop_managed_mlx_lm;
pub use mlx_lm_managed_plan::{
    MlxLmManagedArtifact, MlxLmManagedPhase, MlxLmManagedPlan, MlxLmManagedPlanError,
};
pub use mlx_lm_managed_status::{MlxLmManagedState, MlxLmManagedStatus, inspect_managed_mlx_lm};
pub use ollama::{
    OllamaHostAdapter, OllamaInstallPlanError, OllamaModelPullRefError, OllamaRuntime,
};
pub use ollama_probe::{OllamaBinaryVerification, RuntimeDetection, RuntimeHealth, RuntimeProbe};
pub use process::{ProcessCommand, ProcessOutput, ProcessRunner, SystemProcessRunner};
pub use process_spawn::{ProcessSpawner, SystemProcessSpawner};
pub use productization::{
    GuidedRuntimeSetupPlan, LmStudioProductionAdapter, OllamaInstallerAdapter,
    RuntimeRepairInventory, RuntimeRepairKind, RuntimeRepairPlan,
};
pub use provenance::{RuntimeIntegrityEvidence, RuntimeIntegrityState, RuntimeProvenance};
pub use runtime_lifecycle::{
    RuntimeLifecycleCheckpoint, RuntimeLifecycleFailureClass, RuntimeLifecycleOwnership,
    RuntimeLifecyclePhase, RuntimeLifecycleRecovery, read_runtime_lifecycle,
    write_runtime_lifecycle,
};
pub use status::{
    RuntimeId, RuntimeLifecycleBoundary, RuntimeLifecycleState, RuntimeState, RuntimeStatus,
    VerificationResult,
};
pub use supervisor::{
    RuntimeManagementMode, RuntimeProcessSpec, RuntimeProcessSupervisor, RuntimeSupervisorError,
    RuntimeSupervisorReport,
};
