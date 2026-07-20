#![forbid(unsafe_code)]

mod capabilities;
mod catalog;
mod channel;
mod decision;
mod engine;
mod frontier_catalog;
mod hardware;
mod manifests;
mod seed_catalog;

pub use capabilities::BackendCapabilitySet;
pub use catalog::{BlockedCombination, CompatibilityCatalog, CompatibilityEvidence};
pub use channel::{Channel, ChannelPolicy, LocalOverride};
pub use decision::{CompatibilityDecision, CompatibilityStatus, RecommendationDecision};
pub use engine::{CompatibilityEngine, MatchRequest};
pub use frontier_catalog::{
    CommercialUseState, FrontierCatalogClaimState, FrontierModelClassCatalog,
    FrontierModelClassEntry, FrontierParameterClass, ModelArtifactProvenance,
};
pub use hardware::{AcceleratorConfidence, AcceleratorProfile, HardwareProfile};
pub use manifests::{ModelManifest, RuntimeManifest};
pub use seed_catalog::{ModelLicense, ProductModelSeedCatalog, ProductModelSeedEntry};
