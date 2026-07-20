use desktoplab_agent_session::SessionState;
use desktoplab_backend_services::{
    ApprovalResolution, ApprovalService, ApprovalStore, AuditAction, AuditLogService, AuditQuery,
    AuditStore, BackendRouteCandidate, BackendRouteService, BackendRouteStatus, PluginHost,
    PluginManifest, PluginRouteStatus, RouteApiPolicy, RouteApiRequest, SessionService,
    SessionServiceStore,
};
use desktoplab_policy::PolicyEngine;
use desktoplab_registry::{
    ManifestFamily, ManifestGroup, ManifestStatus, RegistryManifest, RegistryRecommendation,
};
use desktoplab_tool_gateway::{FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn security_denial_path_blocks_every_sensitive_operation_without_side_effects() {
    let workspace = TempDir::new().expect("temp workspace should be created");
    let mut audit = AuditLogService::new(AuditStore::default());
    let mut sessions = SessionService::new(SessionServiceStore::default());
    let session = sessions.create_session("workspace.secure", "backend.ollama");
    sessions.start(session.session_id());

    let mut filesystem =
        FilesystemToolExecutor::new(workspace.path(), PolicyEngine::default_conservative());
    let filesystem_outcome = filesystem.write(
        ".env",
        "OPENAI_API_KEY=sk-test",
        FilesystemApproval::Approved,
    );
    if matches!(filesystem_outcome, FilesystemToolOutcome::Blocked(_)) {
        audit.record_denied(
            AuditAction::ToolExecution,
            "filesystem_write path=.env denied",
        );
    }

    let route = BackendRouteService::new(RouteApiPolicy::local_only()).plan(
        RouteApiRequest::new(&["llm.chat"]),
        vec![BackendRouteCandidate::cloud(
            "backend.openai",
            &["llm.chat"],
        )],
    );
    if route.status() == BackendRouteStatus::Blocked {
        audit.record_denied(
            AuditAction::ProviderEgress,
            "provider_egress local_only_data denied",
        );
    }

    let mut plugins = PluginHost::new("1.0.0");
    plugins.load(PluginManifest::community(
        "plugin.community",
        &["tools.filesystem.write"],
    ));
    let plugin_route = plugins.route("plugin.community");
    if plugin_route.status() == PluginRouteStatus::Blocked {
        audit.record_denied(AuditAction::PluginTrust, "unverified plugin denied");
    }

    let recommendation = RegistryRecommendation::from_group(&ManifestGroup::new(
        ManifestFamily::Model,
        vec![RegistryManifest::new_for_test(
            "model.revoked",
            ManifestFamily::Model,
            ManifestStatus::Revoked,
        )],
    ));
    if recommendation.blocked_reason("model.revoked").is_some() {
        audit.record_denied(AuditAction::ModelDownload, "revoked model manifest denied");
    }

    let mut approvals = ApprovalService::new(ApprovalStore::default());
    let approval = approvals.request(session.session_id(), "filesystem_write:.env");
    let denied = approvals
        .resolve(approval.id(), ApprovalResolution::Deny)
        .expect("approval should resolve");
    if denied.state().is_denied() {
        audit.record_denied(AuditAction::ApprovalDecision, "user denied approval");
        sessions.block(session.session_id(), "approval_denied");
    }

    assert_eq!(
        filesystem_outcome,
        FilesystemToolOutcome::Blocked("local_only_path")
    );
    assert!(!workspace.path().join(".env").exists());
    assert_eq!(route.status(), BackendRouteStatus::Blocked);
    assert!(
        route
            .blocked_reasons()
            .contains(&"egress_blocked".to_string())
    );
    assert_eq!(plugin_route.status(), PluginRouteStatus::Blocked);
    assert!(
        plugin_route
            .reasons()
            .contains(&"unverified_plugin_requires_trust_approval".to_string())
    );
    assert_eq!(
        recommendation.blocked_reason("model.revoked"),
        Some("manifest status is revoked")
    );
    assert_eq!(
        sessions
            .get(session.session_id())
            .expect("session should exist")
            .state(),
        SessionState::Blocked
    );
    assert_eq!(audit.query(AuditQuery::denied()).len(), 5);
}

#[test]
fn security_denial_backend_e2e_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/tests/security_denial_backend_e2e.rs",
        include_str!("security_denial_backend_e2e.rs"),
        180,
    )
    .expect("security denial backend e2e source should stay below the line-count guard");
}
