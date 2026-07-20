use desktoplab_storage::{
    ProductizationRecordKind, ProductizationStateRecord, SecretRejected, SqliteStore, StorageError,
};
use xtask::check_logical_line_limit;

#[test]
fn productization_migration_adds_release_ready_state_tables() {
    let store = SqliteStore::open_in_memory().expect("store should open");

    let report = store.apply_migrations().expect("migrations should pass");
    assert_eq!(report.schema_version(), 4);
    assert_eq!(report.applied_migrations(), 4);
    assert_eq!(store.schema_version().expect("schema version exists"), 4);

    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::RuntimeInventory,
            "runtime.ollama",
            r#"{"state":"not_installed"}"#,
        ))
        .expect("productization state should persist");

    let record = store
        .get_productization_state(ProductizationRecordKind::RuntimeInventory, "runtime.ollama")
        .expect("lookup should pass")
        .expect("record should exist");

    assert_eq!(record.subject_id(), "runtime.ollama");
    assert_eq!(record.payload(), r#"{"state":"not_installed"}"#);
}

#[test]
fn productization_state_rejects_raw_secret_payloads() {
    let store = migrated_store();

    let error = store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::ProviderAccount,
            "provider.openai",
            r#"{"api_key":"sk-live-secret"}"#,
        ))
        .expect_err("raw provider secret payload should be rejected");

    assert_eq!(
        error,
        StorageError::SecretRejected(SecretRejected::new(
            "payload contains forbidden secret-like key"
        ))
    );
    assert!(
        store
            .get_productization_state(ProductizationRecordKind::ProviderAccount, "provider.openai")
            .unwrap()
            .is_none()
    );
}

#[test]
fn productization_state_batch_is_atomic_when_a_record_is_rejected() {
    let store = migrated_store();
    let records = vec![
        ProductizationStateRecord::new(
            ProductizationRecordKind::ApprovalRecord,
            "local",
            r#"{"approvals":[{"state":"approved"}]}"#,
        ),
        ProductizationStateRecord::new(
            ProductizationRecordKind::AgentPendingAction,
            "local",
            r#"{"api_key":"must-not-persist"}"#,
        ),
    ];

    store
        .put_productization_states(&records)
        .expect_err("the complete batch should be rejected");

    assert!(
        store
            .get_productization_state(ProductizationRecordKind::ApprovalRecord, "local")
            .expect("lookup should pass")
            .is_none(),
        "a rejected batch must not persist its valid prefix"
    );
}

#[test]
fn productization_storage_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-storage/src/productization.rs",
        include_str!("../src/productization.rs"),
        220,
    )
    .expect("productization storage source should stay focused");
}

fn migrated_store() -> SqliteStore {
    let store = SqliteStore::open_in_memory().expect("store should open");
    store.apply_migrations().expect("migrations should pass");
    store
}
