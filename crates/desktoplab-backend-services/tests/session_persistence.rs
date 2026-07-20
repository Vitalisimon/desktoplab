use desktoplab_backend_services::{SessionService, SessionServiceStore};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn sessions_survive_sqlite_backed_service_restart() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");

    let first_store = migrated_store(&db_path);
    let mut first_service =
        SessionService::new(SessionServiceStore::with_storage(first_store).expect("store opens"));
    let session = first_service.create_session("workspace.desktoplab", "backend.ollama");
    first_service.start(session.session_id());
    first_service.complete(session.session_id(), "tests passed");

    let second_store = migrated_store(&db_path);
    let restarted = SessionService::new(
        SessionServiceStore::with_storage(second_store).expect("store reopens"),
    );
    let sessions = restarted.list_by_workspace("workspace.desktoplab");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id(), session.session_id());
    assert_eq!(sessions[0].summary(), Some("tests passed"));
}

#[test]
fn session_persistence_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/session_persistence.rs",
        include_str!("session_persistence.rs"),
        140,
    )
    .expect("session persistence test should stay focused");
}

#[test]
fn session_storage_codec_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_storage.rs",
        include_str!("../src/session_storage.rs"),
        220,
    )
    .expect("session storage codec should stay focused");
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
}
