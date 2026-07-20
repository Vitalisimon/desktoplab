use desktoplab_registry::{
    CachedRegistry, ManifestFamily, ManifestGroup, ManifestStatus, RegistryClient, RegistryError,
    RegistryManifest, RegistryRecommendation, RegistrySource, SignatureVerifier,
};
use std::collections::HashMap;
use xtask::check_logical_line_limit;

#[test]
fn runtime_model_backend_and_plugin_manifests_are_separate_families() {
    assert_eq!(ManifestFamily::Runtime.path_segment(), "runtimes");
    assert_eq!(ManifestFamily::Model.path_segment(), "models");
    assert_eq!(ManifestFamily::Backend.path_segment(), "backends");
    assert_eq!(ManifestFamily::Plugin.path_segment(), "plugins");
}

#[test]
fn client_rejects_manifest_with_wrong_family() {
    let source = StaticSource::with_payload(
        ManifestFamily::Runtime,
        signed_group_json(
            ManifestFamily::Runtime,
            &[manifest_json(
                "model.qwen3",
                ManifestFamily::Model,
                ManifestStatus::Stable,
            )],
        ),
    );
    let mut client = RegistryClient::new(source, AcceptingVerifier, CachedRegistry::default());

    let error = client
        .refresh_family(ManifestFamily::Runtime)
        .expect_err("wrong family should be rejected");

    assert_eq!(
        error,
        RegistryError::InvalidManifest(
            "manifest model.qwen3 has family model, expected runtime".to_string()
        )
    );
}

#[test]
fn client_rejects_invalid_schema_before_cache_update() {
    let source = StaticSource::with_payload(
        ManifestFamily::Plugin,
        signed_group_json_raw(
            ManifestFamily::Plugin,
            r#"[{"schema":"wrong/v1","manifest_id":"plugin.bad","manifest_version":"1","family":"plugin","status":"stable","channel":"stable","created_at":"2026-06-25T00:00:00Z","updated_at":"2026-06-25T00:00:00Z","publisher":"desktoplab","content_hash":"sha256:test","compatibility":{},"evidence":{},"policy":{}}]"#,
        ),
    );
    let mut client = RegistryClient::new(source, AcceptingVerifier, CachedRegistry::default());

    let error = client
        .refresh_family(ManifestFamily::Plugin)
        .expect_err("invalid schema should be rejected");

    assert_eq!(
        error,
        RegistryError::InvalidManifest(
            "manifest plugin.bad has unsupported schema wrong/v1".to_string()
        )
    );
    assert!(client.cache().get(ManifestFamily::Plugin).is_none());
}

#[test]
fn signature_verification_boundary_rejects_untrusted_groups() {
    let source = StaticSource::with_payload(
        ManifestFamily::Backend,
        signed_group_json(
            ManifestFamily::Backend,
            &[manifest_json(
                "backend.codex",
                ManifestFamily::Backend,
                ManifestStatus::Stable,
            )],
        ),
    );
    let mut client = RegistryClient::new(source, RejectingVerifier, CachedRegistry::default());

    let error = client
        .refresh_family(ManifestFamily::Backend)
        .expect_err("signature failure should reject group");

    assert_eq!(
        error,
        RegistryError::SignatureRejected("signature rejected by verifier".to_string())
    );
}

#[test]
fn last_known_good_cache_is_used_when_refresh_source_fails() {
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
    let mut client = RegistryClient::new(FailingSource, AcceptingVerifier, cache);

    let group = client
        .refresh_family(ManifestFamily::Runtime)
        .expect("last-known-good should be returned");

    assert_eq!(group.manifests()[0].manifest_id(), "runtime.ollama");
    assert!(group.from_last_known_good());
}

#[test]
fn blocked_and_revoked_entries_override_recommendations() {
    let group = ManifestGroup::new(
        ManifestFamily::Model,
        vec![
            RegistryManifest::new_for_test(
                "model.allowed",
                ManifestFamily::Model,
                ManifestStatus::Stable,
            ),
            RegistryManifest::new_for_test(
                "model.blocked",
                ManifestFamily::Model,
                ManifestStatus::Blocked,
            ),
            RegistryManifest::new_for_test(
                "model.revoked",
                ManifestFamily::Model,
                ManifestStatus::Revoked,
            ),
        ],
    );

    let recommendations = RegistryRecommendation::from_group(&group);

    assert!(recommendations.is_recommended("model.allowed"));
    assert!(!recommendations.is_recommended("model.blocked"));
    assert!(!recommendations.is_recommended("model.revoked"));
    assert_eq!(
        recommendations.blocked_reason("model.blocked"),
        Some("manifest status is blocked")
    );
    assert_eq!(
        recommendations.blocked_reason("model.revoked"),
        Some("manifest status is revoked")
    );
}

#[test]
fn registry_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-registry/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-registry/src/manifest.rs",
            include_str!("../src/manifest.rs"),
            250,
        ),
        (
            "crates/desktoplab-registry/src/client.rs",
            include_str!("../src/client.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("registry source should stay below the initial line-count guard");
    }
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

struct RejectingVerifier;

impl SignatureVerifier for RejectingVerifier {
    fn verify(&self, _payload: &str, _signature: &str) -> Result<(), RegistryError> {
        Err(RegistryError::SignatureRejected(
            "signature rejected by verifier".to_string(),
        ))
    }
}

fn signed_group_json(family: ManifestFamily, manifests: &[String]) -> String {
    signed_group_json_raw(family, &format!("[{}]", manifests.join(",")))
}

fn signed_group_json_raw(family: ManifestFamily, manifests_json: &str) -> String {
    format!(
        r#"{{"schema":"registry.desktoplab.dev/v1","family":"{}","signature":"sig.test","payload":{{"manifests":{manifests_json}}}}}"#,
        family.as_str()
    )
}

fn manifest_json(id: &str, family: ManifestFamily, status: ManifestStatus) -> String {
    format!(
        r#"{{"schema":"registry.desktoplab.dev/v1","manifest_id":"{id}","manifest_version":"1","family":"{}","status":"{}","channel":"stable","created_at":"2026-06-25T00:00:00Z","updated_at":"2026-06-25T00:00:00Z","publisher":"desktoplab","content_hash":"sha256:test","compatibility":{{}},"evidence":{{}},"policy":{{}}}}"#,
        family.as_str(),
        status.as_str()
    )
}
