#![forbid(unsafe_code)]

mod cache;
mod client;
mod error;
mod extension_registry;
mod family;
mod manifest;
mod recommendation;
mod scheduler;
mod signature;
mod source;
mod status;

pub use cache::CachedRegistry;
pub use client::RegistryClient;
pub use error::RegistryError;
pub use extension_registry::{
    ExtensionEvent, ExtensionRecord, ExtensionRegistryError, ExtensionRegistryService,
    ExtensionSourceTrust, ExtensionVersion, InstallTrustPolicy, OwnershipTransfer,
};
pub use family::ManifestFamily;
pub use manifest::{ManifestGroup, RegistryManifest};
pub use recommendation::RegistryRecommendation;
pub use scheduler::{
    RegistryCatalogReadiness, RegistryManualRefreshResult, RegistryRefreshEvent,
    RegistryRefreshEventKind, RegistryRefreshReport, RegistryRefreshScheduler,
    RegistryRefreshStatus,
};
pub use signature::SignatureVerifier;
pub use source::RegistrySource;
pub use status::ManifestStatus;
