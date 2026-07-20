#![forbid(unsafe_code)]

mod catalog;
mod context_window;
mod download;
mod download_execution;
mod frontier_download;
mod frontier_store;
mod inventory;
mod manager;
mod progress;
mod readiness;
mod readiness_service;
mod request_timeout;
mod runtime_adapter;

pub use catalog::{
    ModelFamilyCatalog, ModelLicenseState, ModelParameterClass, ModelRuntimeCompatibility,
    ModelVariant,
};
pub use context_window::AgentContextWindowPolicy;
pub use download::{DownloadPolicy, ModelDownloadPlan, SetupSelection};
pub use download_execution::{
    ModelDownloadCapacity, ModelDownloadError, ModelDownloadEvent, ModelDownloadExecutionPolicy,
    ModelDownloadExecutor, ModelDownloadJob, ModelDownloadMetadata, ModelDownloadProcessResult,
    ModelDownloadState, RuntimeModelDownloadCommand,
};
pub use frontier_download::{
    ArtifactResponse, FrontierArtifactDownload, FrontierDownloadError, FrontierDownloadOutcome,
    FrontierModelStore, HttpsRangeArtifactSource, ResumableArtifactSource,
};
pub use frontier_store::{
    ModelEvictionRecommendation, ModelFootprintTier, ModelStoreCapacity, ModelStoreEntry,
    ModelStoreForecast, ModelStoreInventory,
};
pub use inventory::{
    InMemoryModelInventoryStore, ModelInstallState, ModelInventoryEntry, ModelInventoryService,
    ModelInventorySnapshot, ModelInventorySource, ModelProvenance,
};
pub use manager::{ModelManager, ModelRecommendation};
pub use progress::ModelPullProgress;
pub use readiness::{ModelReadiness, ModelVerification};
pub use readiness_service::{
    InMemoryModelReadinessStore, ModelReadinessService, ModelRouteReadiness, ModelRouteStatus,
    ModelVerificationReport,
};
pub use request_timeout::AgentRequestTimeoutPolicy;
pub use runtime_adapter::{
    HighEndModelRuntimeAdapter, MlxLmModelRuntimeAdapter, ModelRuntimeAdapter,
    ModelRuntimePullResult, ModelRuntimeReadiness, ModelRuntimeReadinessResult,
    OllamaModelRuntimeAdapter, RuntimeModelPullRequest,
};
