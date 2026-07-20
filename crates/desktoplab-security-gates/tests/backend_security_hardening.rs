use desktoplab_acp_plugin::AcpBackendPlugin;
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use desktoplab_policy::{Action, DecisionOutcome, EgressClassification, PolicyEngine};
use desktoplab_registry::{
    ManifestFamily, ManifestGroup, ManifestStatus, RegistryManifest, RegistryRecommendation,
};
use desktoplab_storage::{EventEnvelope, EventStore, RedactionStatus, SqliteStore, StreamKind};

#[test]
fn local_only_categories_never_leave_machine_by_default() {
    let decision = PolicyEngine::default_conservative()
        .evaluate(Action::ProviderEgress(EgressClassification::LocalOnly));

    assert_eq!(decision.outcome(), DecisionOutcome::Denied);
}

#[test]
fn raw_secrets_do_not_appear_in_events_logs_or_metadata() {
    let store = SqliteStore::open_in_memory().expect("store should open");
    store.apply_migrations().expect("migrations should pass");

    let secret_event = EventEnvelope::new(
        "event.secret",
        "session.security",
        StreamKind::Session,
        1,
        "secret.leak",
        r#"{"token":"sk-live-secret"}"#,
    );

    assert!(store.append_event(secret_event).is_err());
    assert!(
        store
            .replay_stream("session.security")
            .expect("stream should replay")
            .is_empty()
    );

    store
        .append_event(
            EventEnvelope::new(
                "event.redacted",
                "session.security",
                StreamKind::Session,
                2,
                "secret.redacted",
                r#"{"token":"[REDACTED]"}"#,
            )
            .with_redaction_status(RedactionStatus::Redacted),
        )
        .expect("redacted payload should store");
    assert!(
        !store.replay_stream("session.security").unwrap()[0]
            .payload()
            .contains("sk-live")
    );
}

#[test]
fn revoked_or_blocked_manifests_prevent_setup_or_execution_recommendation() {
    let group = ManifestGroup::new(
        ManifestFamily::Model,
        vec![
            RegistryManifest::new_for_test(
                "model.revoked",
                ManifestFamily::Model,
                ManifestStatus::Revoked,
            ),
            RegistryManifest::new_for_test(
                "model.blocked",
                ManifestFamily::Model,
                ManifestStatus::Blocked,
            ),
        ],
    );
    let recommendation = RegistryRecommendation::from_group(&group);

    assert!(!recommendation.is_recommended("model.revoked"));
    assert_eq!(
        recommendation.blocked_reason("model.revoked"),
        Some("manifest status is revoked")
    );
    assert!(!recommendation.is_recommended("model.blocked"));
}

#[test]
fn unverified_community_plugins_cannot_silently_escalate() {
    let plugin = AcpBackendPlugin::new_unverified("plugin.acp");
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["llm.chat", "plugin.acp"]),
        vec![plugin.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"unverified_backend_blocked".to_string())
    );
}
