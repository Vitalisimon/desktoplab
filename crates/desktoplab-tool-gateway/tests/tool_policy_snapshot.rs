use desktoplab_policy::{Action, ApprovalMode, DecisionOutcome, PolicyEngine, PolicyReason};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent, ToolOutcome};

#[test]
fn tool_decision_records_layered_policy_snapshot() {
    let mut gateway = ToolGateway::new(
        PolicyEngine::default_conservative()
            .with_approval_mode(ApprovalMode::ApproveWorkspaceWritesForSession),
    );

    assert_eq!(
        gateway.authorize(ToolIntent::git_commit("feat: change")),
        ToolOutcome::ApprovalRequired(Action::GitCommit)
    );

    let record = gateway.audit_records().last().unwrap();
    assert_eq!(
        record.decision().outcome(),
        DecisionOutcome::RequiresApproval
    );
    assert!(record.policy_snapshot().redacted());
    assert_eq!(
        record.policy_snapshot().layers(),
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
fn protected_paths_are_blocked_and_audited_before_execution() {
    let mut gateway = ToolGateway::new(
        PolicyEngine::default_conservative().with_approval_mode(ApprovalMode::FullAccess),
    );

    assert_eq!(
        gateway.authorize(ToolIntent::filesystem_read(".env")),
        ToolOutcome::Blocked("local_only_path".to_string())
    );

    let record = gateway.audit_records().last().unwrap();
    assert_eq!(record.decision().outcome(), DecisionOutcome::Denied);
    assert_eq!(
        record.decision().reason(),
        PolicyReason::LocalOnlyDataBlocked
    );
    assert_eq!(
        record.policy_snapshot().workspace_protection(),
        "workspace_protection:protected_path_blocked"
    );
}

#[test]
fn high_risk_tool_actions_remain_approval_gated_with_snapshots() {
    let mut gateway = ToolGateway::new(
        PolicyEngine::default_conservative().with_approval_mode(ApprovalMode::FullAccess),
    );

    let cases = [
        (
            ToolIntent::terminal("npm install left-pad"),
            Action::DependencyInstall,
            "workspace_protection:dependency_install_confirmation_required",
        ),
        (
            ToolIntent::filesystem_write("package-lock.json"),
            Action::GeneratedArtifactWrite,
            "workspace_protection:generated_artifact_budget",
        ),
        (
            ToolIntent::git_push("origin", "main"),
            Action::GitPush,
            "workspace_protection:git_confirmation_required",
        ),
    ];

    for (intent, action, workspace_layer) in cases {
        assert_eq!(
            gateway.authorize(intent),
            ToolOutcome::ApprovalRequired(action)
        );
        let record = gateway.audit_records().last().unwrap();
        assert_eq!(
            record.decision().outcome(),
            DecisionOutcome::RequiresApproval
        );
        assert_eq!(
            record.policy_snapshot().workspace_protection(),
            workspace_layer
        );
        assert!(record.policy_snapshot().redacted());
    }
}
