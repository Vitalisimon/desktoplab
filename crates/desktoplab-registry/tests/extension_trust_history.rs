use desktoplab_registry::{
    ExtensionRegistryError, ExtensionRegistryService, ExtensionSourceTrust, ExtensionVersion,
    InstallTrustPolicy,
};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;

const DIGEST_V1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const DIGEST_V2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

#[test]
fn immutable_lineage_install_update_rollback_revoke_and_transfer_persist() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("registry.sqlite");
    let store = migrated_store(&database);
    let registry = ExtensionRegistryService::new(&store);
    registry
        .publish(
            "plugin.example",
            "publisher.one",
            version("1.0.0", DIGEST_V1, None, "publisher.one"),
            10,
        )
        .unwrap();
    assert_eq!(
        registry.publish(
            "plugin.example",
            "publisher.one",
            version("1.0.0", DIGEST_V2, None, "publisher.one"),
            11
        ),
        Err(ExtensionRegistryError::ImmutableVersionConflict)
    );
    registry
        .publish(
            "plugin.example",
            "publisher.one",
            version("1.1.0", DIGEST_V2, Some(DIGEST_V1), "publisher.one"),
            20,
        )
        .unwrap();
    let policy = InstallTrustPolicy {
        allow_local_owner: false,
    };
    registry
        .install("plugin.example", DIGEST_V1, "operator", policy, 30)
        .unwrap();
    registry
        .update("plugin.example", DIGEST_V2, "operator", policy, 40)
        .unwrap();
    registry
        .rollback("plugin.example", DIGEST_V1, "operator", policy, 50)
        .unwrap();
    assert_eq!(
        registry.transfer_ownership(
            "plugin.example",
            "publisher.one",
            "publisher.two",
            true,
            false,
            60
        ),
        Err(ExtensionRegistryError::Invalid("transfer_consent_required"))
    );
    registry
        .transfer_ownership(
            "plugin.example",
            "publisher.one",
            "publisher.two",
            true,
            true,
            61,
        )
        .unwrap();
    let revoked = registry
        .revoke("plugin.example", DIGEST_V1, "publisher.two", 70)
        .unwrap();
    assert!(revoked.installed_revoked);
    assert_eq!(
        registry.install("plugin.example", DIGEST_V1, "operator", policy, 80),
        Err(ExtensionRegistryError::VersionRevoked)
    );

    drop(store);
    let reopened = migrated_store(&database);
    let persisted = ExtensionRegistryService::new(&reopened)
        .load("plugin.example")
        .unwrap();
    assert_eq!(persisted.owner_publisher_id, "publisher.two");
    assert_eq!(persisted.versions.len(), 2);
    assert!(
        persisted
            .events
            .iter()
            .any(|event| event.kind == "rolled_back")
    );
}

#[test]
fn source_trust_is_enforced_for_installation() {
    let fixture = TempDir::new().unwrap();
    let store = migrated_store(&fixture.path().join("registry.sqlite"));
    let registry = ExtensionRegistryService::new(&store);
    let mut local = version("1.0.0", DIGEST_V1, None, "publisher.local");
    local.source_trust = ExtensionSourceTrust::LocalOwner;
    registry
        .publish("plugin.local", "publisher.local", local, 10)
        .unwrap();
    assert_eq!(
        registry.install(
            "plugin.local",
            DIGEST_V1,
            "operator",
            InstallTrustPolicy {
                allow_local_owner: false
            },
            20
        ),
        Err(ExtensionRegistryError::TrustDenied)
    );
    registry
        .install(
            "plugin.local",
            DIGEST_V1,
            "operator",
            InstallTrustPolicy {
                allow_local_owner: true,
            },
            21,
        )
        .unwrap();
}

#[test]
fn extension_registry_source_stays_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-registry/src/extension_registry.rs",
        include_str!("../src/extension_registry.rs"),
        380,
    )
    .unwrap();
}

fn version(
    number: &str,
    digest: &str,
    predecessor: Option<&str>,
    publisher: &str,
) -> ExtensionVersion {
    ExtensionVersion {
        version: number.to_string(),
        digest: digest.to_string(),
        predecessor_digest: predecessor.map(ToString::to_string),
        publisher_id: publisher.to_string(),
        source_trust: ExtensionSourceTrust::VerifiedPublisher,
        revoked: false,
    }
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).unwrap();
    store.apply_migrations().unwrap();
    store
}
