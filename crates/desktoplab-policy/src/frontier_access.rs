#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierDeploymentMode {
    SingleUserWorkstation,
    SharedWorkstation,
    TeamNode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceAccess {
    Owner,
    GrantedTeamMember,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierResourceAction {
    UseWorkspace,
    ReadModelCache {
        checksum_verified: bool,
        contains_user_material: bool,
    },
    ResolveApproval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierAccessRequest {
    actor_user_id: String,
    session_owner_user_id: String,
    resource_owner_user_id: String,
    workspace_access: WorkspaceAccess,
    action: FrontierResourceAction,
}

impl FrontierAccessRequest {
    #[must_use]
    pub fn new(
        actor_user_id: impl Into<String>,
        session_owner_user_id: impl Into<String>,
        resource_owner_user_id: impl Into<String>,
        workspace_access: WorkspaceAccess,
        action: FrontierResourceAction,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            session_owner_user_id: session_owner_user_id.into(),
            resource_owner_user_id: resource_owner_user_id.into(),
            workspace_access,
            action,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierAccessReason {
    AllowedOwner,
    AllowedTeamGrant,
    AllowedVerifiedSharedCache,
    SessionOwnerMismatch,
    WorkspaceOwnerMismatch,
    WorkspaceGrantMissing,
    CacheNotVerified,
    CacheContainsUserMaterial,
    ApprovalOwnerMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrontierAccessDecision {
    allowed: bool,
    reason: FrontierAccessReason,
}

impl FrontierAccessDecision {
    #[must_use]
    pub fn allowed(self) -> bool {
        self.allowed
    }

    #[must_use]
    pub fn reason(self) -> FrontierAccessReason {
        self.reason
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrontierAccessPolicy {
    mode: FrontierDeploymentMode,
}

impl FrontierAccessPolicy {
    #[must_use]
    pub fn new(mode: FrontierDeploymentMode) -> Self {
        Self { mode }
    }

    #[must_use]
    pub fn evaluate(self, request: &FrontierAccessRequest) -> FrontierAccessDecision {
        match request.action {
            FrontierResourceAction::UseWorkspace => self.workspace_decision(request),
            FrontierResourceAction::ReadModelCache {
                checksum_verified,
                contains_user_material,
            } => self.cache_decision(request, checksum_verified, contains_user_material),
            FrontierResourceAction::ResolveApproval => self.approval_decision(request),
        }
    }

    fn workspace_decision(self, request: &FrontierAccessRequest) -> FrontierAccessDecision {
        if request.actor_user_id != request.session_owner_user_id {
            return denied(FrontierAccessReason::SessionOwnerMismatch);
        }
        if request.actor_user_id == request.resource_owner_user_id
            && request.workspace_access == WorkspaceAccess::Owner
        {
            return allowed(FrontierAccessReason::AllowedOwner);
        }
        if self.mode == FrontierDeploymentMode::TeamNode
            && request.workspace_access == WorkspaceAccess::GrantedTeamMember
        {
            return allowed(FrontierAccessReason::AllowedTeamGrant);
        }
        if request.workspace_access == WorkspaceAccess::Denied {
            denied(FrontierAccessReason::WorkspaceGrantMissing)
        } else {
            denied(FrontierAccessReason::WorkspaceOwnerMismatch)
        }
    }

    fn cache_decision(
        self,
        request: &FrontierAccessRequest,
        checksum_verified: bool,
        contains_user_material: bool,
    ) -> FrontierAccessDecision {
        if request.actor_user_id != request.session_owner_user_id {
            return denied(FrontierAccessReason::SessionOwnerMismatch);
        }
        if request.actor_user_id == request.resource_owner_user_id {
            return allowed(FrontierAccessReason::AllowedOwner);
        }
        if contains_user_material {
            return denied(FrontierAccessReason::CacheContainsUserMaterial);
        }
        if !checksum_verified {
            return denied(FrontierAccessReason::CacheNotVerified);
        }
        if matches!(
            self.mode,
            FrontierDeploymentMode::SharedWorkstation | FrontierDeploymentMode::TeamNode
        ) {
            allowed(FrontierAccessReason::AllowedVerifiedSharedCache)
        } else {
            denied(FrontierAccessReason::WorkspaceOwnerMismatch)
        }
    }

    fn approval_decision(self, request: &FrontierAccessRequest) -> FrontierAccessDecision {
        if request.actor_user_id == request.session_owner_user_id {
            allowed(FrontierAccessReason::AllowedOwner)
        } else {
            denied(FrontierAccessReason::ApprovalOwnerMismatch)
        }
    }
}

fn allowed(reason: FrontierAccessReason) -> FrontierAccessDecision {
    FrontierAccessDecision {
        allowed: true,
        reason,
    }
}

fn denied(reason: FrontierAccessReason) -> FrontierAccessDecision {
    FrontierAccessDecision {
        allowed: false,
        reason,
    }
}
