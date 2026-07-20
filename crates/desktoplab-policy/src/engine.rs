use desktoplab_domain::ApprovalMode;

use crate::{
    Action, ApprovalRisk, DecisionOutcome, EgressClassification, PolicyDecision, PolicyReason,
    ProviderEgressContext,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderEgressPolicy {
    Deny,
    RequireApproval,
    Allow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyEngine {
    provider_egress: ProviderEgressPolicy,
    approval_mode: ApprovalMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyLayerSnapshot {
    default_policy: &'static str,
    approval_mode: &'static str,
    workspace_protection: &'static str,
    backend_capability: &'static str,
    plugin_layer: &'static str,
    sandbox_layer: &'static str,
    redacted: bool,
}

impl PolicyLayerSnapshot {
    #[must_use]
    pub fn layers(&self) -> Vec<&'static str> {
        vec![
            self.default_policy,
            self.approval_mode,
            self.workspace_protection,
            self.backend_capability,
            self.plugin_layer,
            self.sandbox_layer,
        ]
    }

    #[must_use]
    pub fn redacted(&self) -> bool {
        self.redacted
    }

    #[must_use]
    pub fn workspace_protection(&self) -> &'static str {
        self.workspace_protection
    }
}

impl PolicyEngine {
    #[must_use]
    pub fn default_conservative() -> Self {
        Self {
            provider_egress: ProviderEgressPolicy::RequireApproval,
            approval_mode: ApprovalMode::RequireApproval,
        }
    }

    #[must_use]
    pub fn with_provider_egress(mut self, provider_egress: ProviderEgressPolicy) -> Self {
        self.provider_egress = provider_egress;
        self
    }

    #[must_use]
    pub fn with_approval_mode(mut self, approval_mode: ApprovalMode) -> Self {
        self.approval_mode = approval_mode;
        self
    }

    #[must_use]
    pub fn approval_mode(&self) -> ApprovalMode {
        self.approval_mode
    }

    #[must_use]
    pub fn layer_snapshot(&self, action: Action) -> PolicyLayerSnapshot {
        PolicyLayerSnapshot {
            default_policy: "default_policy:conservative_local_first",
            approval_mode: approval_mode_layer(self.approval_mode),
            workspace_protection: workspace_protection_layer(action),
            backend_capability: "backend_capability:declared_by_execution_backend",
            plugin_layer: "plugin_layer:reserved_fail_closed",
            sandbox_layer: "sandbox_layer:reserved_fail_closed",
            redacted: true,
        }
    }

    #[must_use]
    pub fn evaluate(&self, action: Action) -> PolicyDecision {
        let decision = match action {
            Action::ProtectedWorkspaceAccess => PolicyDecision::new(
                action,
                DecisionOutcome::Denied,
                ApprovalRisk::High,
                PolicyReason::LocalOnlyDataBlocked,
            ),
            Action::FilesystemRead
            | Action::ProcessControl
            | Action::GitRead
            | Action::CheckpointCreate
            | Action::McpInvoke
            | Action::Clarification
            | Action::AgentControl => PolicyDecision::new(
                action,
                DecisionOutcome::AllowedAutomatic,
                ApprovalRisk::Low,
                PolicyReason::LocalToolAllowedByPolicy,
            ),
            Action::FilesystemWrite
            | Action::TerminalCommand
            | Action::ProcessStart
            | Action::TestRun
            | Action::GitCommit => approval_required(action, ApprovalRisk::Medium),
            Action::DependencyInstall => PolicyDecision::new(
                action,
                DecisionOutcome::RequiresApproval,
                ApprovalRisk::High,
                PolicyReason::DependencyInstallRequiresApproval,
            ),
            Action::GeneratedArtifactWrite => PolicyDecision::new(
                action,
                DecisionOutcome::RequiresApproval,
                ApprovalRisk::High,
                PolicyReason::GeneratedArtifactBudgetRequiresApproval,
            ),
            Action::GitPush => approval_required(action, ApprovalRisk::High),
            Action::ModelDownload | Action::RuntimeInstall => PolicyDecision::new(
                action,
                DecisionOutcome::AllowedAutomatic,
                ApprovalRisk::Low,
                PolicyReason::SetupActionAllowedAfterPlanSelection,
            ),
            Action::ProviderEgress(classification) => {
                self.evaluate_provider_egress(action, classification)
            }
            Action::ProviderEgressWithAccount(context) => {
                self.evaluate_provider_egress_context(action, context)
            }
        };
        self.apply_approval_mode(decision)
    }

    fn apply_approval_mode(&self, decision: PolicyDecision) -> PolicyDecision {
        if decision.outcome() != DecisionOutcome::RequiresApproval {
            return decision;
        }
        if requires_explicit_user_confirmation(&decision) {
            return decision;
        }
        match (self.approval_mode, decision.risk()) {
            (ApprovalMode::RequireApproval, _) => decision,
            (ApprovalMode::ApproveWorkspaceWritesForSession, _)
                if decision.action() == Action::FilesystemWrite =>
            {
                PolicyDecision::new(
                    decision.action(),
                    DecisionOutcome::AllowedAutomatic,
                    decision.risk(),
                    PolicyReason::ApprovalModeAllowedRoutine,
                )
            }
            (ApprovalMode::ApproveWorkspaceWritesForSession, _) => decision,
            (ApprovalMode::ApproveForMe, ApprovalRisk::High) => decision,
            (ApprovalMode::ApproveForMe, _) => PolicyDecision::new(
                decision.action(),
                DecisionOutcome::AllowedAutomatic,
                decision.risk(),
                PolicyReason::ApprovalModeAllowedRoutine,
            ),
            (ApprovalMode::FullAccess, _) => PolicyDecision::new(
                decision.action(),
                DecisionOutcome::AllowedAutomatic,
                decision.risk(),
                PolicyReason::ApprovalModeFullAccess,
            ),
        }
    }

    fn evaluate_provider_egress_context(
        &self,
        action: Action,
        context: ProviderEgressContext,
    ) -> PolicyDecision {
        if context.classification() == EgressClassification::LocalOnly {
            return PolicyDecision::new(
                action,
                DecisionOutcome::Denied,
                ApprovalRisk::High,
                PolicyReason::LocalOnlyDataBlocked,
            );
        }
        if context.requires_billing_fallback_approval() {
            return PolicyDecision::new(
                action,
                DecisionOutcome::RequiresApproval,
                ApprovalRisk::High,
                PolicyReason::BillingFallbackRequiresApproval,
            );
        }
        self.evaluate_provider_egress(action, context.classification())
    }

    fn evaluate_provider_egress(
        &self,
        action: Action,
        classification: EgressClassification,
    ) -> PolicyDecision {
        if classification == EgressClassification::LocalOnly {
            return PolicyDecision::new(
                action,
                DecisionOutcome::Denied,
                ApprovalRisk::High,
                PolicyReason::LocalOnlyDataBlocked,
            );
        }

        match self.provider_egress {
            ProviderEgressPolicy::Deny => PolicyDecision::new(
                action,
                DecisionOutcome::Denied,
                ApprovalRisk::High,
                PolicyReason::ProviderEgressDeniedByPolicy,
            ),
            ProviderEgressPolicy::RequireApproval => PolicyDecision::new(
                action,
                DecisionOutcome::RequiresApproval,
                ApprovalRisk::High,
                PolicyReason::ProviderEgressRequiresApproval,
            ),
            ProviderEgressPolicy::Allow => PolicyDecision::new(
                action,
                DecisionOutcome::AllowedAutomatic,
                ApprovalRisk::Medium,
                PolicyReason::ProviderEgressAllowedByPolicy,
            ),
        }
    }
}

fn requires_explicit_user_confirmation(decision: &PolicyDecision) -> bool {
    if matches!(decision.action(), Action::GitCommit | Action::GitPush) {
        return true;
    }
    matches!(
        decision.reason(),
        PolicyReason::ProviderEgressRequiresApproval
            | PolicyReason::BillingFallbackRequiresApproval
            | PolicyReason::DependencyInstallRequiresApproval
            | PolicyReason::GeneratedArtifactBudgetRequiresApproval
    )
}

fn approval_required(action: Action, risk: ApprovalRisk) -> PolicyDecision {
    PolicyDecision::new(
        action,
        DecisionOutcome::RequiresApproval,
        risk,
        PolicyReason::SensitiveMutationRequiresApproval,
    )
}

fn approval_mode_layer(mode: ApprovalMode) -> &'static str {
    match mode {
        ApprovalMode::RequireApproval => "approval_mode:require_approval",
        ApprovalMode::ApproveForMe => "approval_mode:approve_for_me",
        ApprovalMode::ApproveWorkspaceWritesForSession => {
            "approval_mode:approve_workspace_writes_for_session"
        }
        ApprovalMode::FullAccess => "approval_mode:full_access",
    }
}

fn workspace_protection_layer(action: Action) -> &'static str {
    match action {
        Action::ProtectedWorkspaceAccess => "workspace_protection:protected_path_blocked",
        Action::GeneratedArtifactWrite => "workspace_protection:generated_artifact_budget",
        Action::GitCommit | Action::GitPush => "workspace_protection:git_confirmation_required",
        Action::DependencyInstall => {
            "workspace_protection:dependency_install_confirmation_required"
        }
        _ => "workspace_protection:workspace_scoped",
    }
}
