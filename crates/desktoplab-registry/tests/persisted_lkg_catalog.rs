use desktoplab_registry::{
    CachedRegistry, ManifestFamily, ManifestStatus, RegistryClient, RegistryError, RegistrySource,
    SignatureVerifier,
};
use std::collections::HashMap;

#[test]
fn registry_cache_survives_restart_and_serves_last_known_good_offline() {
    let path = std::env::temp_dir().join(format!(
        "desktoplab-registry-lkg-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let source = StaticSource::with_payload(
        ManifestFamily::Model,
        signed_group_json(
            ManifestFamily::Model,
            &[manifest_json(
                "model.qwen-coder-7b-q4",
                ManifestFamily::Model,
                ManifestStatus::Stable,
            )],
        ),
    );
    let mut online = RegistryClient::new(
        source,
        AcceptingVerifier,
        CachedRegistry::with_storage_path(&path).expect("cache should open"),
    );

    let refreshed = online
        .refresh_family(ManifestFamily::Model)
        .expect("online refresh should succeed");
    assert!(!refreshed.from_last_known_good());

    let mut offline = RegistryClient::new(
        FailingSource,
        AcceptingVerifier,
        CachedRegistry::with_storage_path(&path).expect("cache should reload"),
    );
    let lkg = offline
        .refresh_family(ManifestFamily::Model)
        .expect("persisted last-known-good should be returned");

    assert_eq!(lkg.manifests()[0].manifest_id(), "model.qwen-coder-7b-q4");
    assert!(lkg.from_last_known_good());

    let _ = std::fs::remove_file(path);
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
