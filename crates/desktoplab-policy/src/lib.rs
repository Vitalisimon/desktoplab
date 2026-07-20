#![forbid(unsafe_code)]

mod action;
mod decision;
mod engine;
mod frontier_access;

pub use action::{Action, EgressAccountMode, EgressClassification, ProviderEgressContext};
pub use decision::{
    ApprovalRisk, DecisionOutcome, PolicyDecision, PolicyDecisionRecord, PolicyReason,
};
pub use desktoplab_domain::ApprovalMode;
pub use engine::{PolicyEngine, PolicyLayerSnapshot, ProviderEgressPolicy};
pub use frontier_access::{
    FrontierAccessDecision, FrontierAccessPolicy, FrontierAccessReason, FrontierAccessRequest,
    FrontierDeploymentMode, FrontierResourceAction, WorkspaceAccess,
};
