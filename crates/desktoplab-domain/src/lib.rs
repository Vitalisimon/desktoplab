#![forbid(unsafe_code)]

mod backend;
mod deep_link;
mod descriptors;
mod ids;
mod policy;
mod profiles;
mod route;
mod session;

pub use backend::{ExecutionBackend, ExecutionBackendKind, Provider, ProviderKind};
pub use deep_link::{DeepLinkAction, DeepLinkError, DesktopLabDeepLink};
pub use descriptors::{
    AccountMode, CompatibilityManifest, PluginDescriptor, ProviderAccountDescriptor,
    RegistryDescriptor, RuntimeDescriptor, ToolDescriptor,
};
pub use ids::{
    AgentProfileId, ExecutionBackendId, ModelProfileId, PluginId, ProviderId, RegistryId,
    RuntimeId, SessionId, ToolId, WorkspaceId,
};
pub use policy::{ApprovalMode, ApprovalPolicy, MemoryScope, Policy};
pub use profiles::{AgentProfile, HardwareProfile, ModelProfile};
pub use route::{BackendCapability, ExecutionRoute};
pub use session::{Session, SessionOwner, Workspace};
