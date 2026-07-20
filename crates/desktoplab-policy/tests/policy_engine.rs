use desktoplab_policy::{
    Action, ApprovalRisk, DecisionOutcome, EgressAccountMode, EgressClassification,
    PolicyDecisionRecord, PolicyEngine, PolicyReason, ProviderEgressContext, ProviderEgressPolicy,
};
use xtask::check_logical_line_limit;

#[test]
fn default_policy_requires_approval_for_write_terminal_and_git_mutations() {
    let policy = PolicyEngine::default_conservative();

    assert_requires_approval(policy.evaluate(Action::FilesystemWrite));
    assert_requires_approval(policy.evaluate(Action::GeneratedArtifactWrite));
    assert_requires_approval(policy.evaluate(Action::TerminalCommand));
    assert_requires_approval(policy.evaluate(Action::DependencyInstall));
    assert_requires_approval(policy.evaluate(Action::GitCommit));
    assert_requires_approval(policy.evaluate(Action::GitPush));
}

#[test]
fn approval_modes_do_not_auto_allow_dependency_installs_or_generated_artifacts() {
    for mode in [
        desktoplab_policy::ApprovalMode::ApproveForMe,
        desktoplab_policy::ApprovalMode::FullAccess,
    ] {
        let policy = PolicyEngine::default_conservative().with_approval_mode(mode);
        let install = policy.evaluate(Action::DependencyInstall);
        let generated = policy.evaluate(Action::GeneratedArtifactWrite);

        assert_eq!(install.outcome(), DecisionOutcome::RequiresApproval);
        assert_eq!(
            install.reason(),
            PolicyReason::DependencyInstallRequiresApproval
        );
        assert_eq!(generated.outcome(), DecisionOutcome::RequiresApproval);
        assert_eq!(
            generated.reason(),
            PolicyReason::GeneratedArtifactBudgetRequiresApproval
        );
    }
}

#[test]
fn setup_download_actions_are_automatic_but_still_recorded() {
    let policy = PolicyEngine::default_conservative();

    assert_allowed_automatic(policy.evaluate(Action::ModelDownload));
    assert_allowed_automatic(policy.evaluate(Action::RuntimeInstall));
    assert_allowed_automatic(policy.evaluate(Action::AgentControl));
}

#[test]
fn provider_egress_is_policy_dependent() {
    let deny_policy =
        PolicyEngine::default_conservative().with_provider_egress(ProviderEgressPolicy::Deny);
    let approval_policy = PolicyEngine::default_conservative()
        .with_provider_egress(ProviderEgressPolicy::RequireApproval);
    let allow_policy =
        PolicyEngine::default_conservative().with_provider_egress(ProviderEgressPolicy::Allow);

    assert_eq!(
        deny_policy
            .evaluate(Action::ProviderEgress(EgressClassification::SafeToEgress))
            .outcome(),
        DecisionOutcome::Denied
    );
    assert_requires_approval(
        approval_policy.evaluate(Action::ProviderEgress(EgressClassification::SafeToEgress)),
    );
    assert_allowed_automatic(
        allow_policy.evaluate(Action::ProviderEgress(EgressClassification::SafeToEgress)),
    );
}

#[test]
fn local_only_provider_egress_fails_closed_even_when_egress_policy_allows() {
    let policy =
        PolicyEngine::default_conservative().with_provider_egress(ProviderEgressPolicy::Allow);
    let decision = policy.evaluate(Action::ProviderEgress(EgressClassification::LocalOnly));

    assert_eq!(decision.outcome(), DecisionOutcome::Denied);
    assert_eq!(decision.reason(), PolicyReason::LocalOnlyDataBlocked);
    assert!(!decision.can_execute_without_approval());
}

#[test]
fn subscription_to_api_billing_fallback_requires_explicit_approval() {
    let policy =
        PolicyEngine::default_conservative().with_provider_egress(ProviderEgressPolicy::Allow);
    let decision = policy.evaluate(Action::ProviderEgressWithAccount(
        ProviderEgressContext::new(
            EgressClassification::SafeToEgress,
            EgressAccountMode::SubscriptionAccount,
        )
        .with_fallback_account_mode(EgressAccountMode::ApiKeyBilling),
    ));

    assert_eq!(decision.outcome(), DecisionOutcome::RequiresApproval);
    assert_eq!(
        decision.reason(),
        PolicyReason::BillingFallbackRequiresApproval
    );
    assert!(
        decision
            .approval_disclosure()
            .contains("subscription_account")
    );
    assert!(decision.approval_disclosure().contains("api_key_billing"));
}

#[test]
fn full_access_does_not_bypass_external_egress_confirmation() {
    let policy = PolicyEngine::default_conservative()
        .with_provider_egress(ProviderEgressPolicy::RequireApproval)
        .with_approval_mode(desktoplab_policy::ApprovalMode::FullAccess);

    let provider_egress =
        policy.evaluate(Action::ProviderEgress(EgressClassification::SafeToEgress));
    assert_eq!(provider_egress.outcome(), DecisionOutcome::RequiresApproval);
    assert_eq!(
        provider_egress.reason(),
        PolicyReason::ProviderEgressRequiresApproval
    );

    let billing_fallback = policy.evaluate(Action::ProviderEgressWithAccount(
        ProviderEgressContext::new(
            EgressClassification::SafeToEgress,
            EgressAccountMode::SubscriptionAccount,
        )
        .with_fallback_account_mode(EgressAccountMode::ApiKeyBilling),
    ));
    assert_eq!(
        billing_fallback.outcome(),
        DecisionOutcome::RequiresApproval
    );
    assert_eq!(
        billing_fallback.reason(),
        PolicyReason::BillingFallbackRequiresApproval
    );
}

#[test]
fn full_access_does_not_bypass_git_commit_or_push_confirmation() {
    let policy = PolicyEngine::default_conservative()
        .with_approval_mode(desktoplab_policy::ApprovalMode::FullAccess);

    assert_requires_approval(policy.evaluate(Action::GitCommit));
    assert_requires_approval(policy.evaluate(Action::GitPush));
}

#[test]
fn approval_decisions_include_risk_and_auditable_reason() {
    let policy = PolicyEngine::default_conservative();
    let decision = policy.evaluate(Action::GitPush);

    assert_eq!(decision.action(), Action::GitPush);
    assert_eq!(decision.outcome(), DecisionOutcome::RequiresApproval);
    assert_eq!(decision.risk(), ApprovalRisk::High);
    assert_eq!(
        decision.reason(),
        PolicyReason::SensitiveMutationRequiresApproval
    );
    assert_eq!(
        decision.to_audit_record(),
        PolicyDecisionRecord::new(
            Action::GitPush,
            DecisionOutcome::RequiresApproval,
            ApprovalRisk::High,
            PolicyReason::SensitiveMutationRequiresApproval,
        )
    );
}

#[test]
fn policy_layer_snapshot_is_redacted_and_names_active_layers() {
    let policy = PolicyEngine::default_conservative()
        .with_approval_mode(desktoplab_policy::ApprovalMode::ApproveWorkspaceWritesForSession);

    let snapshot = policy.layer_snapshot(Action::GitCommit);

    assert!(snapshot.redacted());
    assert_eq!(
        snapshot.layers(),
        vec![
            "default_policy:conservative_local_first",
            "approval_mode:approve_workspace_writes_for_session",
            "workspace_protection:git_confirmation_required",
            "backend_capability:declared_by_execution_backend",
            "plugin_layer:reserved_fail_closed",
            "sandbox_layer:reserved_fail_closed",
        ]
    );
}

#[test]
fn protected_workspace_access_fails_closed_with_snapshot_layer() {
    let policy = PolicyEngine::default_conservative()
        .with_approval_mode(desktoplab_policy::ApprovalMode::FullAccess);

    let decision = policy.evaluate(Action::ProtectedWorkspaceAccess);
    let snapshot = policy.layer_snapshot(Action::ProtectedWorkspaceAccess);

    assert_eq!(decision.outcome(), DecisionOutcome::Denied);
    assert_eq!(decision.reason(), PolicyReason::LocalOnlyDataBlocked);
    assert_eq!(
        snapshot.workspace_protection(),
        "workspace_protection:protected_path_blocked"
    );
}

#[test]
fn denied_actions_fail_closed() {
    let policy =
        PolicyEngine::default_conservative().with_provider_egress(ProviderEgressPolicy::Deny);
    let decision = policy.evaluate(Action::ProviderEgress(EgressClassification::SafeToEgress));

    assert_eq!(decision.outcome(), DecisionOutcome::Denied);
    assert!(!decision.can_execute_without_approval());
    assert!(!decision.can_request_approval());
}

#[test]
fn policy_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-policy/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("policy lib should stay below the initial line-count guard");
}

fn assert_requires_approval(decision: desktoplab_policy::PolicyDecision) {
    assert_eq!(decision.outcome(), DecisionOutcome::RequiresApproval);
    assert!(decision.can_request_approval());
    assert!(!decision.can_execute_without_approval());
}

fn assert_allowed_automatic(decision: desktoplab_policy::PolicyDecision) {
    assert_eq!(decision.outcome(), DecisionOutcome::AllowedAutomatic);
    assert!(!decision.can_request_approval());
    assert!(decision.can_execute_without_approval());
}
