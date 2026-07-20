use desktoplab_storage::{SecretRejected, SettingRecord, SettingValue, SqliteStore, StorageError};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use xtask::check_logical_line_limit;

#[test]
fn settings_persist_across_store_reopen() {
    let database_path = unique_database_path("settings-reopen");

    {
        let store = migrated_store_at(&database_path);
        store
            .put_setting(SettingRecord::new(
                "policy.default_execution",
                SettingValue::String("local_first".to_string()),
            ))
            .expect("setting should persist");
    }

    let reopened = migrated_store_at(&database_path);
    let setting = reopened
        .get_setting("policy.default_execution")
        .expect("setting lookup should pass")
        .expect("setting should exist after reopen");

    assert_eq!(setting.key(), "policy.default_execution");
    assert_eq!(
        setting.value(),
        &SettingValue::String("local_first".to_string())
    );
}

#[test]
fn raw_secret_values_are_rejected() {
    let store = migrated_memory_store();

    let error = store
        .put_setting(SettingRecord::new(
            "provider.openai.api_key",
            SettingValue::String("sk-live-secret".to_string()),
        ))
        .expect_err("raw secret settings should be rejected");

    assert_eq!(
        error,
        StorageError::SecretRejected(SecretRejected::new(
            "settings store accepts secret references only"
        ))
    );
    assert!(
        store
            .get_setting("provider.openai.api_key")
            .unwrap()
            .is_none()
    );
}

#[test]
fn secret_references_can_be_stored_without_raw_values() {
    let store = migrated_memory_store();

    store
        .put_setting(SettingRecord::new(
            "provider.openai.api_key",
            SettingValue::SecretReference("vault://provider/openai/api-key".to_string()),
        ))
        .expect("secret references should persist");

    let setting = store
        .get_setting("provider.openai.api_key")
        .expect("setting lookup should pass")
        .expect("secret reference setting should exist");

    assert_eq!(
        setting.value(),
        &SettingValue::SecretReference("vault://provider/openai/api-key".to_string())
    );
}

#[test]
fn settings_migration_is_deterministic() {
    let store = SqliteStore::open_in_memory().expect("store should open");

    let first = store
        .apply_migrations()
        .expect("first migration should pass");
    let second = store
        .apply_migrations()
        .expect("second migration should pass");

    assert_eq!(first.schema_version(), 4);
    assert_eq!(first.applied_migrations(), 4);
    assert_eq!(second.schema_version(), 4);
    assert_eq!(second.applied_migrations(), 0);
    assert_eq!(store.schema_version().expect("schema version exists"), 4);
}

#[test]
fn settings_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-storage/src/settings.rs",
        include_str!("../src/settings.rs"),
        260,
    )
    .expect("settings source should stay below the initial line-count guard");
}

fn migrated_memory_store() -> SqliteStore {
    let store = SqliteStore::open_in_memory().expect("store should open");
    store.apply_migrations().expect("migrations should pass");
    store
}

fn migrated_store_at(path: &PathBuf) -> SqliteStore {
    let store = SqliteStore::open(path).expect("store should open");
    store.apply_migrations().expect("migrations should pass");
    store
}

fn unique_database_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("desktoplab-{label}-{nanos}.sqlite"))
}
