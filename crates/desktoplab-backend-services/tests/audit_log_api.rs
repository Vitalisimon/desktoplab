use desktoplab_backend_services::{AuditAction, AuditLogService, AuditQuery, AuditStore};
use xtask::check_logical_line_limit;

#[test]
fn audit_log_redacts_sensitive_values() {
    let mut audit = AuditLogService::new(AuditStore::default());

    audit.record(
        AuditAction::ProviderEgress,
        "api_key=sk-secret token=abc123",
    );

    let records = audit.query(AuditQuery::all());
    assert_eq!(records[0].details(), "api_key=[REDACTED] token=[REDACTED]");
}

#[test]
fn local_audit_transparency_snapshot_is_redacted_and_api_facing() {
    let mut audit = AuditLogService::new(AuditStore::default());

    audit.record(
        AuditAction::ProviderEgress,
        "api_key=sk-secret Authorization=Bearer raw-token cookie=sessionid=raw session=raw-session",
    );
    audit.record_denied(
        AuditAction::ToolExecution,
        "terminal denied bearer raw-bearer-token provider_token=raw-provider-token",
    );

    let snapshot = audit.transparency_snapshot(AuditQuery::all(), 10);

    assert_eq!(snapshot.scope, "local_single_user");
    assert_eq!(snapshot.records.len(), 2);
    assert_eq!(snapshot.records[0].action, "provider_egress");
    assert_eq!(snapshot.records[0].outcome, "allowed");
    assert_eq!(snapshot.records[1].outcome, "denied");
    assert!(snapshot.records[0].redacted_details.contains("[REDACTED]"));
    assert!(snapshot.redacted_export.contains("provider_egress allowed"));
    assert!(snapshot.redacted_export.contains("tool_execution denied"));
    for forbidden in [
        "sk-secret",
        "raw-token",
        "sessionid=raw",
        "raw-session",
        "raw-bearer-token",
        "raw-provider-token",
    ] {
        assert!(
            !snapshot.redacted_export.contains(forbidden),
            "audit export leaked {forbidden}"
        );
    }
}

#[test]
fn policy_and_approval_decisions_are_queryable() {
    let mut audit = AuditLogService::new(AuditStore::default());
    audit.record(
        AuditAction::PolicyDecision,
        "filesystem write requires approval",
    );
    audit.record(AuditAction::ApprovalDecision, "denied by user");

    let policy = audit.query(AuditQuery::action(AuditAction::PolicyDecision));
    let approval = audit.query(AuditQuery::action(AuditAction::ApprovalDecision));

    assert_eq!(policy.len(), 1);
    assert_eq!(approval.len(), 1);
    assert_eq!(approval[0].details(), "denied by user");
}

#[test]
fn denied_actions_are_auditable() {
    let mut audit = AuditLogService::new(AuditStore::default());

    audit.record_denied(AuditAction::ToolExecution, "terminal denied");

    let denied = audit.query(AuditQuery::denied());
    assert_eq!(denied.len(), 1);
    assert_eq!(denied[0].action(), AuditAction::ToolExecution);
}

#[test]
fn audit_records_survive_service_restart() {
    let store = AuditStore::default();
    let mut first = AuditLogService::new(store.clone());
    first.record(AuditAction::RuntimeInstall, "queued runtime.ollama");

    let restarted = AuditLogService::new(store);

    assert_eq!(restarted.query(AuditQuery::all()).len(), 1);
}

#[test]
fn audit_log_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/audit.rs",
        include_str!("../src/audit.rs"),
        260,
    )
    .expect("audit log api source should stay below the line-count guard");
}
