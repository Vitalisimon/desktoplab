use desktoplab_storage::{SqliteStore, migration_plan};
use xtask::check_logical_line_limit;

#[test]
fn every_migration_declares_operator_visible_contract() {
    let plan = migration_plan();

    assert_eq!(plan.len(), 4);
    for migration in plan {
        assert!(migration.id().starts_with("migration-"));
        assert!(migration.version() > 0);
        assert!(migration.checksum().starts_with(migration.id()));
        assert!(!migration.description().is_empty());
        assert!(matches!(
            migration.reversibility_class(),
            "forward_only" | "reversible" | "manual_restore"
        ));
    }
}

#[test]
fn migration_report_includes_bounded_statuses() {
    let store = SqliteStore::open_in_memory().expect("store should open");

    let report = store.apply_migrations().expect("migrations should apply");

    assert_eq!(report.schema_version(), 4);
    assert_eq!(report.applied_migrations(), 4);
    assert_eq!(report.migrations().len(), migration_plan().len());
    assert!(
        report
            .migrations()
            .iter()
            .all(|status| status.operator_status() == "applied")
    );
}

#[test]
fn migration_source_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-storage/src/migration.rs",
            include_str!("../src/migration.rs"),
            260,
        ),
        (
            "crates/desktoplab-storage/tests/storage_migration_discipline.rs",
            include_str!("storage_migration_discipline.rs"),
            140,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("migration discipline files should stay focused");
    }
}
