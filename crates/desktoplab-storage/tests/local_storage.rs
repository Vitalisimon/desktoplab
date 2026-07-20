use desktoplab_storage::{
    EventEnvelope, EventStore, MigrationReport, RedactionStatus, SecretRejected, SqliteStore,
    StorageError, StreamKind, migration_plan,
};
use xtask::check_logical_line_limit;

#[test]
fn migrations_are_idempotent_and_report_schema_version() {
    let store = SqliteStore::open_in_memory().expect("store should open");

    assert_migration_report(
        store
            .apply_migrations()
            .expect("first migration should pass"),
        4,
    );
    assert_migration_report(
        store
            .apply_migrations()
            .expect("second migration should pass"),
        0,
    );
    assert_eq!(store.schema_version().expect("schema version exists"), 4);
}

fn assert_migration_report(report: MigrationReport, applied_migrations: u32) {
    assert_eq!(report.schema_version(), 4);
    assert_eq!(report.applied_migrations(), applied_migrations);
    assert_eq!(report.migrations().len(), migration_plan().len());
}

#[test]
fn events_can_be_appended_and_replayed_in_sequence_order() {
    let store = migrated_store();

    store
        .append_event(EventEnvelope::new(
            "event.002",
            "session.001",
            StreamKind::Session,
            2,
            "agent.step.completed",
            r#"{"summary":"done"}"#,
        ))
        .expect("event 2 should append");
    store
        .append_event(EventEnvelope::new(
            "event.001",
            "session.001",
            StreamKind::Session,
            1,
            "agent.step.started",
            r#"{"summary":"started"}"#,
        ))
        .expect("event 1 should append");

    let events = store
        .replay_stream("session.001")
        .expect("stream should replay");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id(), "event.001");
    assert_eq!(events[0].sequence(), 1);
    assert_eq!(events[1].event_id(), "event.002");
    assert_eq!(events[1].sequence(), 2);
}

#[test]
fn append_rejects_secret_like_payloads_before_persistence() {
    let store = migrated_store();
    let event = EventEnvelope::new(
        "event.secret",
        "session.001",
        StreamKind::Session,
        1,
        "provider.auth.failed",
        r#"{"api_key":"sk-live-secret"}"#,
    );

    let error = store
        .append_event(event)
        .expect_err("raw secret-like payload should be rejected");

    assert_eq!(
        error,
        StorageError::SecretRejected(SecretRejected::new(
            "payload contains forbidden secret-like key"
        ))
    );
    assert!(store.replay_stream("session.001").unwrap().is_empty());
}

#[test]
fn redacted_payloads_can_be_persisted_with_explicit_redaction_status() {
    let store = migrated_store();
    let event = EventEnvelope::new(
        "event.redacted",
        "session.001",
        StreamKind::Session,
        1,
        "provider.auth.failed",
        r#"{"api_key":"[REDACTED]"}"#,
    )
    .with_redaction_status(RedactionStatus::Redacted);

    store
        .append_event(event)
        .expect("redacted event should append");

    let events = store.replay_stream("session.001").unwrap();

    assert_eq!(events[0].redaction_status(), RedactionStatus::Redacted);
    assert_eq!(events[0].payload(), r#"{"api_key":"[REDACTED]"}"#);
}

#[test]
fn storage_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-storage/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("storage lib should stay below the initial line-count guard");
}

fn migrated_store() -> SqliteStore {
    let store = SqliteStore::open_in_memory().expect("store should open");
    store.apply_migrations().expect("migrations should pass");
    store
}
