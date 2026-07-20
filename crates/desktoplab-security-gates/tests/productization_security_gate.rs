use desktoplab_backend_services::{
    AuditAction, AuditLogService, AuditQuery, AuditStore, PluginManifestLoader,
    PluginPermissionEngine, PluginProductizationHost,
};
use desktoplab_policy::{Action, DecisionOutcome, EgressClassification, PolicyEngine};
use desktoplab_security_gates::{
    ArtifactVerification, ProductizationSecurityGate, ProductizationSecurityGateInput,
};

#[test]
fn productization_security_gate_fails_closed_for_unverified_artifacts_and_local_only_egress() {
    let gate = ProductizationSecurityGate::default();
    let report = gate.evaluate(ProductizationSecurityGateInput {
        binary: ArtifactVerification::Unsigned,
        model: ArtifactVerification::ChecksumMismatch,
        plugin_verified: false,
        provider_egress: EgressClassification::LocalOnly,
        protected_path: ".env".into(),
    });

    assert!(report.is_denied());
    assert!(report.reasons().contains(&"unsigned_binary".to_string()));
    assert!(
        report
            .reasons()
            .contains(&"model_verification_failed".to_string())
    );
    assert!(report.reasons().contains(&"unverified_plugin".to_string()));
    assert!(
        report
            .reasons()
            .contains(&"provider_egress_denied".to_string())
    );
    assert!(
        report
            .reasons()
            .contains(&"protected_path_local_only".to_string())
    );
}

#[test]
fn security_denial_e2e_covers_provider_runtime_model_plugin_workspace_and_git_paths() {
    let policy = PolicyEngine::default_conservative();
    assert_eq!(
        policy
            .evaluate(Action::ProviderEgress(EgressClassification::LocalOnly))
            .outcome(),
        DecisionOutcome::Denied
    );

    let loader = PluginManifestLoader::new("1.0.0");
    let manifest = loader
        .load_from_str(
            r#"{"plugin_id":"plugin.tools","contract_version":"1","trust":"unverified","permissions":["tool.filesystem.write"],"hooks":["tool"]}"#,
        )
        .unwrap();
    let mut host = PluginProductizationHost::new("1.0.0");
    host.load_manifest(manifest).unwrap();
    assert!(
        PluginPermissionEngine::default()
            .authorize(&host, "plugin.tools")
            .is_blocked()
    );

    let gate = ProductizationSecurityGate::default().evaluate(ProductizationSecurityGateInput {
        binary: ArtifactVerification::Verified,
        model: ArtifactVerification::Revoked,
        plugin_verified: false,
        provider_egress: EgressClassification::LocalOnly,
        protected_path: ".git/config".into(),
    });
    assert!(
        gate.reasons()
            .contains(&"model_verification_failed".to_string())
    );
    assert!(
        gate.reasons()
            .contains(&"protected_path_local_only".to_string())
    );
}

#[test]
fn redaction_tests_cover_diagnostics_events_and_audit_export() {
    let mut audit = AuditLogService::new(AuditStore::default());
    audit.record_denied(
        AuditAction::ProviderEgress,
        "api_key=sk-live-secret token=secret",
    );
    let exported = audit.export_redacted(AuditQuery::denied());

    assert!(exported.contains("[REDACTED]"));
    assert!(!exported.contains("sk-live-secret"));
    assert!(!exported.contains("token=secret"));
}
