use crate::Action;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionOutcome {
    RequiresApproval,
    AllowedAutomatic,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyReason {
    LocalToolAllowedByPolicy,
    SensitiveMutationRequiresApproval,
    SetupActionAllowedAfterPlanSelection,
    ProviderEgressAllowedByPolicy,
    ProviderEgressRequiresApproval,
    ProviderEgressDeniedByPolicy,
    BillingFallbackRequiresApproval,
    LocalOnlyDataBlocked,
    ApprovalModeAllowedRoutine,
    ApprovalModeFullAccess,
    DependencyInstallRequiresApproval,
    GeneratedArtifactBudgetRequiresApproval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    action: Action,
    outcome: DecisionOutcome,
    risk: ApprovalRisk,
    reason: PolicyReason,
}

impl PolicyDecision {
    #[must_use]
    pub fn new(
        action: Action,
        outcome: DecisionOutcome,
        risk: ApprovalRisk,
        reason: PolicyReason,
    ) -> Self {
        Self {
            action,
            outcome,
            risk,
            reason,
        }
    }

    #[must_use]
    pub fn action(&self) -> Action {
        self.action
    }

    #[must_use]
    pub fn outcome(&self) -> DecisionOutcome {
        self.outcome
    }

    #[must_use]
    pub fn risk(&self) -> ApprovalRisk {
        self.risk
    }

    #[must_use]
    pub fn reason(&self) -> PolicyReason {
        self.reason
    }

    #[must_use]
    pub fn can_execute_without_approval(&self) -> bool {
        self.outcome == DecisionOutcome::AllowedAutomatic
    }

    #[must_use]
    pub fn can_request_approval(&self) -> bool {
        self.outcome == DecisionOutcome::RequiresApproval
    }

    #[must_use]
    pub fn to_audit_record(&self) -> PolicyDecisionRecord {
        PolicyDecisionRecord::new(self.action, self.outcome, self.risk, self.reason)
    }

    #[must_use]
    pub fn approval_disclosure(&self) -> String {
        match self.action {
            Action::ProviderEgressWithAccount(context) => {
                let mut disclosure = format!("account_mode={}", context.account_mode().as_str());
                if let Some(fallback) = context.fallback_account_mode() {
                    disclosure.push_str(" fallback_account_mode=");
                    disclosure.push_str(fallback.as_str());
                }
                disclosure
            }
            Action::ProviderEgress(_) => "provider_egress_account_mode=unknown".to_string(),
            _ => format!("policy_reason={:?}", self.reason),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecisionRecord {
    action: Action,
    outcome: DecisionOutcome,
    risk: ApprovalRisk,
    reason: PolicyReason,
}

impl PolicyDecisionRecord {
    #[must_use]
    pub fn new(
        action: Action,
        outcome: DecisionOutcome,
        risk: ApprovalRisk,
        reason: PolicyReason,
    ) -> Self {
        Self {
            action,
            outcome,
            risk,
            reason,
        }
    }

    #[must_use]
    pub fn outcome(&self) -> DecisionOutcome {
        self.outcome
    }

    #[must_use]
    pub fn action(&self) -> Action {
        self.action
    }

    #[must_use]
    pub fn reason(&self) -> PolicyReason {
        self.reason
    }
}
