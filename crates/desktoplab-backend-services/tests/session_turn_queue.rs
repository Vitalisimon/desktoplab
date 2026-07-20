use desktoplab_backend_services::{SessionService, SessionServiceStore};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn fifo_turns_survive_restart() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("turns.sqlite");
    let storage = SqliteStore::open(&db).unwrap();
    storage.apply_migrations().unwrap();
    let mut service = SessionService::new(SessionServiceStore::with_storage(storage).unwrap());
    let session = service.create_session("workspace.test", "backend.test");
    service.enqueue_turn(session.session_id(), "first");
    service.enqueue_turn(session.session_id(), "second");
    assert_eq!(
        service
            .claim_next_turn(session.session_id())
            .unwrap()
            .prompt(),
        "first"
    );
    drop(service);

    let storage = SqliteStore::open(&db).unwrap();
    storage.apply_migrations().unwrap();
    let mut restarted = SessionService::new(SessionServiceStore::with_storage(storage).unwrap());
    let recovered = restarted.claim_next_turn(session.session_id()).unwrap();
    assert_eq!(recovered.prompt(), "first");
    assert!(restarted.complete_turn(session.session_id(), recovered.turn_id()));
    assert_eq!(
        restarted
            .claim_next_turn(session.session_id())
            .unwrap()
            .prompt(),
        "second"
    );
}

#[test]
fn cancellation_is_cooperative_before_bounded_force() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.test", "backend.test");
    service.request_cancel(session.session_id(), 1_000, 500);
    assert_eq!(
        service.cancellation_state(session.session_id()).as_deref(),
        Some("requested")
    );
    assert!(!service.force_cancel_if_due(session.session_id(), 1_499));
    assert!(service.force_cancel_if_due(session.session_id(), 1_500));
    assert_eq!(
        service.cancellation_state(session.session_id()).as_deref(),
        Some("forced")
    );
    service.request_cancel(session.session_id(), 2_000, 500);
    service.acknowledge_cancel(session.session_id());
    assert!(!service.force_cancel_if_due(session.session_id(), 3_000));
}

#[test]
fn queue_test_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_turns.rs",
        include_str!("../src/session_turns.rs"),
        220,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/session_turn_queue.rs",
        include_str!("session_turn_queue.rs"),
        120,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_queue_service.rs",
        include_str!("../src/session_queue_service.rs"),
        120,
    )
    .unwrap();
}
