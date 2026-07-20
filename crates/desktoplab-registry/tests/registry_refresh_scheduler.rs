use desktoplab_registry::{
    CachedRegistry, ManifestFamily, ManifestGroup, ManifestStatus, RegistryCatalogReadiness,
    RegistryClient, RegistryError, RegistryManifest, RegistryRefreshEventKind,
    RegistryRefreshScheduler, RegistrySource, SignatureVerifier,
};
use std::collections::HashMap;
use xtask::check_logical_line_limit;

#[test]
fn startup_refresh_uses_last_known_good_when_source_fails() {
    let mut cache = CachedRegistry::default();
    cache
        .store(ManifestGroup::new(
            ManifestFamily::Runtime,
            vec![RegistryManifest::new_for_test(
                "runtime.ollama",
                ManifestFamily::Runtime,
                ManifestStatus::Stable,
            )],
        ))
        .expect("cache store should succeed");
    let mut scheduler =
        RegistryRefreshScheduler::new(RegistryClient::new(FailingSource, AcceptingVerifier, cache));

    let report = scheduler.startup_refresh([ManifestFamily::Runtime]);

    assert_eq!(report.readiness(), RegistryCatalogReadiness::Degraded);
    assert!(
        report
            .group(ManifestFamily::Runtime)
            .expect("runtime group should exist")
            .from_last_known_good()
    );
    assert_eq!(
        report.event_kinds(),
        vec![
            RegistryRefreshEventKind::JobStarted,
            RegistryRefreshEventKind::FamilyDegraded,
            RegistryRefreshEventKind::JobCompleted,
        ]
    );
    let status = report.status();
    assert_eq!(status.readiness, RegistryCatalogReadiness::Degraded);
    assert!(status.last_known_good_available);
    assert_eq!(
        status.degraded_reasons,
        vec!["Using last-known-good runtime catalog because refresh is unavailable.".to_string()]
    );
}

#[test]
fn manual_refresh_emits_ordered_job_events() {
    let source = StaticSource::with_payload(
        ManifestFamily::Runtime,
        signed_group_json(
            ManifestFamily::Runtime,
            &[manifest_json(
                "runtime.ollama",
                ManifestFamily::Runtime,
                ManifestStatus::Stable,
            )],
        ),
    );
    let mut scheduler = RegistryRefreshScheduler::new(RegistryClient::new(
        source,
        AcceptingVerifier,
        CachedRegistry::default(),
    ));

    let report = scheduler.manual_refresh("job.registry.manual", [ManifestFamily::Runtime]);

    assert_eq!(report.readiness(), RegistryCatalogReadiness::Ready);
    assert_eq!(
        report.event_kinds(),
        vec![
            RegistryRefreshEventKind::JobQueued,
            RegistryRefreshEventKind::JobStarted,
            RegistryRefreshEventKind::FamilyRefreshed,
            RegistryRefreshEventKind::JobCompleted,
        ]
    );
    assert_eq!(
        report.event_sequences(),
        vec![1, 2, 3, 4],
        "manual refresh events should be stable and ordered"
    );
}

#[test]
fn no_safe_catalog_blocks_setup_recommendations() {
    let mut scheduler = RegistryRefreshScheduler::new(RegistryClient::new(
        FailingSource,
        AcceptingVerifier,
        CachedRegistry::default(),
    ));

    let report = scheduler.startup_refresh([ManifestFamily::Model]);

    assert_eq!(report.readiness(), RegistryCatalogReadiness::Blocked);
    assert_eq!(
        report
            .recommendations(ManifestFamily::Model)
            .expect_err("setup recommendations need a safe catalog"),
        RegistryError::NoSafeCatalog("model catalog is unavailable".to_string())
    );
    let result = report.manual_refresh_result();
    assert_eq!(result.job_id, None);
    assert_eq!(
        result.blocked_reason,
        Some("No safe compatibility catalog is available.".to_string())
    );
}

#[test]
fn blocked_fresh_manifests_override_stale_recommendations() {
    let mut cache = CachedRegistry::default();
    cache
        .store(ManifestGroup::new(
            ManifestFamily::Model,
            vec![RegistryManifest::new_for_test(
                "model.qwen3",
                ManifestFamily::Model,
                ManifestStatus::Stable,
            )],
        ))
        .expect("cache store should succeed");
    let source = StaticSource::with_payload(
        ManifestFamily::Model,
        signed_group_json(
            ManifestFamily::Model,
            &[manifest_json(
                "model.qwen3",
                ManifestFamily::Model,
                ManifestStatus::Blocked,
            )],
        ),
    );
    let mut scheduler =
        RegistryRefreshScheduler::new(RegistryClient::new(source, AcceptingVerifier, cache));

    let report = scheduler.manual_refresh("job.registry.manual", [ManifestFamily::Model]);
    let recommendations = report
        .recommendations(ManifestFamily::Model)
        .expect("fresh model catalog should produce recommendations");

    assert!(!recommendations.is_recommended("model.qwen3"));
    assert_eq!(
        recommendations.blocked_reason("model.qwen3"),
        Some("manifest status is blocked")
    );
}

#[test]
fn registry_scheduler_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-registry/src/scheduler.rs",
        include_str!("../src/scheduler.rs"),
        280,
    )
    .expect("registry scheduler source should stay below the initial line-count guard");
}

#[derive(Clone)]
struct StaticSource {
    payloads: HashMap<ManifestFamily, String>,
}

impl StaticSource {
    fn with_payload(family: ManifestFamily, payload: String) -> Self {
        Self {
            payloads: HashMap::from([(family, payload)]),
        }
    }
}

impl RegistrySource for StaticSource {
    fn fetch_family(&self, family: ManifestFamily) -> Result<String, RegistryError> {
        self.payloads
            .get(&family)
            .cloned()
            .ok_or_else(|| RegistryError::SourceUnavailable("missing fixture".to_string()))
    }
}

struct FailingSource;

impl RegistrySource for FailingSource {
    fn fetch_family(&self, _family: ManifestFamily) -> Result<String, RegistryError> {
        Err(RegistryError::SourceUnavailable("offline".to_string()))
    }
}

struct AcceptingVerifier;

impl SignatureVerifier for AcceptingVerifier {
    fn verify(&self, _payload: &str, _signature: &str) -> Result<(), RegistryError> {
        Ok(())
    }
}

fn signed_group_json(family: ManifestFamily, manifests: &[String]) -> String {
    format!(
        r#"{{"schema":"registry.desktoplab.dev/v1","family":"{}","signature":"sig.test","payload":{{"manifests":[{}]}}}}"#,
        family.as_str(),
        manifests.join(",")
    )
}

fn manifest_json(id: &str, family: ManifestFamily, status: ManifestStatus) -> String {
    format!(
        r#"{{"schema":"registry.desktoplab.dev/v1","manifest_id":"{id}","manifest_version":"1","family":"{}","status":"{}","channel":"stable","created_at":"2026-06-25T00:00:00Z","updated_at":"2026-06-25T00:00:00Z","publisher":"desktoplab","content_hash":"sha256:test","compatibility":{{}},"evidence":{{}},"policy":{{}}}}"#,
        family.as_str(),
        status.as_str()
    )
}
