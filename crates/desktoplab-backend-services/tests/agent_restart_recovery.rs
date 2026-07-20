use desktoplab_agent_session::SessionState;
use desktoplab_backend_services::{SessionContinuation, SessionService, SessionServiceStore};
use xtask::check_logical_line_limit;

#[test]
fn blocked_session_replays_with_cursor_after_process_restart() {
    let store = SessionServiceStore::default();
    let mut first_service = SessionService::new(store.clone());
    let session = first_service.create_session("workspace.1", "backend.ollama");
    first_service.start(session.session_id());
    first_service.block(session.session_id(), "waiting for approval");

    let restarted_service = SessionService::new(store);
    let snapshot = restarted_service
        .recover(session.session_id())
        .expect("blocked session should recover after restart");

    assert_eq!(snapshot.session().state(), SessionState::Blocked);
    assert_eq!(snapshot.cursor().event_count(), 3);
    assert!(snapshot.cursor().can_continue_after_approval());
}

#[test]
fn approval_after_restart_resumes_blocked_session() {
    let store = SessionServiceStore::default();
    let mut first_service = SessionService::new(store.clone());
    let session = first_service.create_session("workspace.1", "backend.ollama");
    first_service.start(session.session_id());
    first_service.block(session.session_id(), "waiting for approval");

    let mut restarted_service = SessionService::new(store);
    let outcome = restarted_service.resume_after_approval(session.session_id());
    let snapshot = restarted_service
        .recover(session.session_id())
        .expect("resumed session should still recover");

    assert_eq!(outcome, SessionContinuation::Resumed);
    assert_eq!(snapshot.session().state(), SessionState::Running);
    assert_eq!(snapshot.cursor().event_count(), 4);
}

#[test]
fn cancellation_after_restart_prevents_continuation() {
    let store = SessionServiceStore::default();
    let mut first_service = SessionService::new(store.clone());
    let session = first_service.create_session("workspace.1", "backend.ollama");
    first_service.start(session.session_id());
    first_service.block(session.session_id(), "waiting for approval");

    let mut restarted_service = SessionService::new(store);
    restarted_service.cancel(session.session_id(), "user stopped run");

    let outcome = restarted_service.resume_after_approval(session.session_id());

    assert_eq!(
        outcome,
        SessionContinuation::Terminal(SessionState::Cancelled)
    );
    assert_eq!(
        restarted_service
            .recover(session.session_id())
            .unwrap()
            .session()
            .state(),
        SessionState::Cancelled
    );
}

#[test]
fn duplicate_resume_after_restart_is_idempotent() {
    let store = SessionServiceStore::default();
    let mut first_service = SessionService::new(store.clone());
    let session = first_service.create_session("workspace.1", "backend.ollama");
    first_service.start(session.session_id());
    first_service.block(session.session_id(), "waiting for approval");

    let mut restarted_service = SessionService::new(store);
    let first = restarted_service.resume_after_approval(session.session_id());
    let second = restarted_service.resume_after_approval(session.session_id());
    let snapshot = restarted_service.recover(session.session_id()).unwrap();

    assert_eq!(first, SessionContinuation::Resumed);
    assert_eq!(second, SessionContinuation::AlreadyRunning);
    assert_eq!(snapshot.cursor().event_count(), 4);
}

#[test]
fn recovery_snapshot_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/agent_restart_recovery.rs",
        include_str!("agent_restart_recovery.rs"),
        180,
    )
    .expect("restart recovery tests should stay below the line-count guard");

    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/sessions.rs",
        include_str!("../src/sessions.rs"),
        360,
    )
    .expect("session service source should stay below the line-count guard");
}
