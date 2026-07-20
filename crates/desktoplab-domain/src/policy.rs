#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalPolicy {
    Conservative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalMode {
    RequireApproval,
    ApproveForMe,
    ApproveWorkspaceWritesForSession,
    FullAccess,
}

impl ApprovalMode {
    pub const ALL: [Self; 4] = [
        Self::RequireApproval,
        Self::ApproveForMe,
        Self::ApproveWorkspaceWritesForSession,
        Self::FullAccess,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequireApproval => "require_approval",
            Self::ApproveForMe => "approve_for_me",
            Self::ApproveWorkspaceWritesForSession => "approve_workspace_writes_for_session",
            Self::FullAccess => "full_access",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::RequireApproval => "Ask for approval",
            Self::ApproveForMe => "Approve routine actions",
            Self::ApproveWorkspaceWritesForSession => "Allow workspace writes",
            Self::FullAccess => "Full local access",
        }
    }

    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::RequireApproval => {
                "Recommended for small local models and careful first runs. Agent terminal commands, commits and pushes stop for approval."
            }
            Self::ApproveForMe => {
                "DesktopLab can approve routine local steps while provider egress, pushes and protected data still stop."
            }
            Self::ApproveWorkspaceWritesForSession => {
                "DesktopLab can approve workspace file writes for this session while terminal commands, commits, pushes and protected data still stop."
            }
            Self::FullAccess => {
                "DesktopLab reduces routine prompts while commits, pushes, provider egress and hard security blocks still stop."
            }
        }
    }

    #[must_use]
    pub fn from_stable_str(value: &str) -> Option<Self> {
        match value {
            "require_approval" => Some(Self::RequireApproval),
            "approve_for_me" => Some(Self::ApproveForMe),
            "approve_workspace_writes_for_session" => Some(Self::ApproveWorkspaceWritesForSession),
            "full_access" => Some(Self::FullAccess),
            _ => None,
        }
    }
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::RequireApproval
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Policy {
    approval_policy: ApprovalPolicy,
}

impl Policy {
    #[must_use]
    pub fn new(approval_policy: ApprovalPolicy) -> Self {
        Self { approval_policy }
    }

    #[must_use]
    pub fn approval_policy(&self) -> ApprovalPolicy {
        self.approval_policy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryScope {
    Workspace,
}
