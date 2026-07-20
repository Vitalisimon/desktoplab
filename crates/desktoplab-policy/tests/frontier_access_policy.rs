use desktoplab_policy::{
    FrontierAccessPolicy, FrontierAccessReason, FrontierAccessRequest, FrontierDeploymentMode,
    FrontierResourceAction, WorkspaceAccess,
};
use xtask::check_logical_line_limit;

#[test]
fn shared_workstation_keeps_workspace_and_session_ownership_isolated() {
    let policy = FrontierAccessPolicy::new(FrontierDeploymentMode::SharedWorkstation);
    let decision = policy.evaluate(&request(
        "user.b",
        "user.b",
        "user.a",
        WorkspaceAccess::GrantedTeamMember,
        FrontierResourceAction::UseWorkspace,
    ));

    assert!(!decision.allowed());
    assert_eq!(
        decision.reason(),
        FrontierAccessReason::WorkspaceOwnerMismatch
    );
}

#[test]
fn team_node_accepts_explicit_workspace_grant_but_not_foreign_approval() {
    let policy = FrontierAccessPolicy::new(FrontierDeploymentMode::TeamNode);
    let workspace = policy.evaluate(&request(
        "user.b",
        "user.b",
        "user.a",
        WorkspaceAccess::GrantedTeamMember,
        FrontierResourceAction::UseWorkspace,
    ));
    let approval = policy.evaluate(&request(
        "user.admin",
        "user.b",
        "user.b",
        WorkspaceAccess::Owner,
        FrontierResourceAction::ResolveApproval,
    ));

    assert!(workspace.allowed());
    assert_eq!(workspace.reason(), FrontierAccessReason::AllowedTeamGrant);
    assert!(!approval.allowed());
    assert_eq!(
        approval.reason(),
        FrontierAccessReason::ApprovalOwnerMismatch
    );
}

#[test]
fn shared_model_cache_requires_verified_non_user_material() {
    let policy = FrontierAccessPolicy::new(FrontierDeploymentMode::SharedWorkstation);
    let shared = policy.evaluate(&cache_request(true, false));
    let unverified = policy.evaluate(&cache_request(false, false));
    let contaminated = policy.evaluate(&cache_request(true, true));

    assert!(shared.allowed());
    assert_eq!(
        shared.reason(),
        FrontierAccessReason::AllowedVerifiedSharedCache
    );
    assert_eq!(unverified.reason(), FrontierAccessReason::CacheNotVerified);
    assert_eq!(
        contaminated.reason(),
        FrontierAccessReason::CacheContainsUserMaterial
    );
}

#[test]
fn single_user_mode_never_shares_foreign_cache() {
    let policy = FrontierAccessPolicy::new(FrontierDeploymentMode::SingleUserWorkstation);
    assert!(!policy.evaluate(&cache_request(true, false)).allowed());
}

#[test]
fn frontier_access_policy_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-policy/src/frontier_access.rs",
        include_str!("../src/frontier_access.rs"),
        220,
    )
    .expect("frontier access policy should remain focused");
}

fn cache_request(checksum_verified: bool, contains_user_material: bool) -> FrontierAccessRequest {
    request(
        "user.b",
        "user.b",
        "user.a",
        WorkspaceAccess::Owner,
        FrontierResourceAction::ReadModelCache {
            checksum_verified,
            contains_user_material,
        },
    )
}

fn request(
    actor: &str,
    session_owner: &str,
    resource_owner: &str,
    workspace_access: WorkspaceAccess,
    action: FrontierResourceAction,
) -> FrontierAccessRequest {
    FrontierAccessRequest::new(
        actor,
        session_owner,
        resource_owner,
        workspace_access,
        action,
    )
}
