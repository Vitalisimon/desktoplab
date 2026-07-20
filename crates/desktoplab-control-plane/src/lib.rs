#![forbid(unsafe_code)]

mod acp_bridge;
mod agent_completion_grounding;
mod agent_execution_obligations;
mod agent_failure;
mod agent_model_adapter;
mod api;
mod app_state;
mod auth;
mod canonical_tool_executor;
mod canonical_tool_files;
mod canonical_tool_git;
mod canonical_tool_process;
mod canonical_tool_search;
mod client;
mod discovery;
mod error;
mod execution_backend_capabilities;
mod execution_external_routes;
mod execution_route_labels;
mod execution_route_options;
mod execution_routes;
mod execution_tool_calling;
mod execution_tool_calling_evidence;
mod frontier_scheduler;
mod frontier_setup_preview;
mod high_end_runtime_routes;
mod http;
mod lifecycle;
mod mcp_tokens;
mod model_inventory_routes;
mod model_pull_ref_validation;
mod model_route_errors;
mod model_routes;
mod origin;
mod provider_accounts;
mod provider_bridge_routes;
mod provider_routes;
mod readiness_state;
mod router;
mod router_payloads;
mod runtime_routes;
mod setup_pipeline;
mod setup_routes;
mod setup_state;
mod terminal_routes;
mod verification_evidence;
mod workspace_files;

pub use api::{ApiSurface, VersionInfo};
pub use auth::{AuthDecision, LocalApiAuth, LocalAuthToken};
pub use canonical_tool_executor::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};
pub use client::ClientKind;
pub use discovery::{
    DiscoveryError, DiscoveryPermissionState, DiscoveryProcessState, LocalApiDiscoveryDocument,
    LocalApiDiscoveryPath, LocalApiDiscoveryWriter,
};
pub use error::{ControlPlaneError, ErrorCode};
pub use execution_tool_calling::{
    BackendToolProtocolClass, BackendToolProtocolHealth, ToolProtocolError,
    backend_tool_protocol_class, normalize_backend_tool_output,
};
pub use frontier_scheduler::{
    FrontierPartitionKind, FrontierResourceLease, FrontierResourcePartition,
    FrontierResourceScheduler, FrontierScheduleDecision, FrontierScheduleRequest,
};
pub use http::{ControlPlaneHttpServer, HttpServerConfig, HttpServerError, HttpServerHandle};
pub use lifecycle::{
    ControlPlane, ControlPlaneHealth, ControlPlaneReadiness, ControlPlaneStatus, LifecycleState,
    ReadinessState, ShutdownMode,
};
pub use origin::{CorsDecision, LocalApiRequestOrigin, OriginPolicy};
pub use readiness_state::BackendReadinessState;
pub use router::{ApiRouteResponse, LocalApiRouter};
pub use setup_pipeline::{SetupPipeline, SetupPipelineState};

use std::sync::{Arc, Mutex};

pub fn bind_default_local_api_server(port: u16) -> Result<ControlPlaneHttpServer, HttpServerError> {
    bind_authenticated_local_api_server(port, LocalAuthToken::for_desktop_session())
}

pub fn bind_authenticated_local_api_server(
    port: u16,
    token: LocalAuthToken,
) -> Result<ControlPlaneHttpServer, HttpServerError> {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane
        .lock()
        .expect("control plane lock should not be poisoned")
        .mark_ready();
    ControlPlaneHttpServer::bind(
        HttpServerConfig::loopback(port)?.with_auth(LocalApiAuth::required(token)),
        control_plane,
    )
}

pub fn bind_unsafe_dev_local_api_server(
    port: u16,
) -> Result<ControlPlaneHttpServer, HttpServerError> {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane
        .lock()
        .expect("control plane lock should not be poisoned")
        .mark_ready();
    ControlPlaneHttpServer::bind(HttpServerConfig::loopback(port)?, control_plane)
}
