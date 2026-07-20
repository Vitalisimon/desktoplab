use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use tempfile::TempDir;

#[test]
fn setup_state_payload_survives_sqlite_reopen() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");

    let store = SqliteStore::open(&db_path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
        .put_productization_state(ProductizationStateRecord::new(
            ProductizationRecordKind::SetupState,
            "local",
            r#"{"state":"in_progress","runtimeId":"runtime.ollama","modelId":"model.qwen-coder"}"#,
        ))
        .expect("setup state should persist");

    let resumed = SqliteStore::open(&db_path).expect("store should reopen");
    resumed.apply_migrations().expect("migrations should apply");
    let state = resumed
        .get_productization_state(ProductizationRecordKind::SetupState, "local")
        .expect("read should work")
        .expect("setup state should exist");

    assert_eq!(state.kind(), ProductizationRecordKind::SetupState);
    assert!(state.payload().contains("in_progress"));
}

#[test]
fn unknown_productization_record_kind_does_not_fall_back_to_valid_domain_kind() {
    assert_eq!(
        ProductizationRecordKind::from_storage("future_unmigrated_kind"),
        ProductizationRecordKind::Unknown
    );
}
