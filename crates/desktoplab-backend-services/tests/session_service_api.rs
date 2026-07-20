use desktoplab_agent_session::{SessionOwner, SessionState};
use desktoplab_backend_services::{SessionService, SessionServiceStore};
use xtask::check_logical_line_limit;

#[test]
fn desktoplab_owns_sessions_even_with_external_backend() {
    let mut service = SessionService::new(SessionServiceStore::default());

    let session = service.create_session("workspace.1", "backend.codex.app-server");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.codex.app-server");
}

#[test]
fn replay_reconstructs_state_after_restart() {
    let store = SessionServiceStore::default();
    let mut first_service = SessionService::new(store.clone());
    let session = first_service.create_session("workspace.1", "backend.ollama");
    first_service.start(session.session_id());
    first_service.complete(session.session_id(), "changed README");

    let restarted_service = SessionService::new(store);
    let replayed = restarted_service
        .replay(session.session_id())
        .expect("session replay should work");

    assert_eq!(replayed.state(), SessionState::Completed);
    assert_eq!(replayed.summary(), Some("changed README"));
}

#[test]
fn blocked_failed_cancelled_and_completed_states_remain_distinct() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let blocked = service.create_session("workspace.1", "backend.ollama");
    let failed = service.create_session("workspace.1", "backend.ollama");
    let cancelled = service.create_session("workspace.1", "backend.ollama");
    let completed = service.create_session("workspace.1", "backend.ollama");

    service.block(blocked.session_id(), "approval required");
    service.fail(failed.session_id(), "tool failed");
    service.cancel(cancelled.session_id(), "user cancelled");
    service.complete(completed.session_id(), "done");

    assert_eq!(
        service.get(blocked.session_id()).unwrap().state(),
        SessionState::Blocked
    );
    assert_eq!(
        service.get(failed.session_id()).unwrap().state(),
        SessionState::Failed
    );
    assert_eq!(
        service.get(cancelled.session_id()).unwrap().state(),
        SessionState::Cancelled
    );
    assert_eq!(
        service.get(completed.session_id()).unwrap().state(),
        SessionState::Completed
    );
}

#[test]
fn session_summary_is_persisted_and_sessions_list_by_workspace() {
    let store = SessionServiceStore::default();
    let mut service = SessionService::new(store.clone());
    let session = service.create_session("workspace.42", "backend.ollama");
    service.complete(session.session_id(), "tests passed");

    let restarted = SessionService::new(store);
    let sessions = restarted.list_by_workspace("workspace.42");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].summary(), Some("tests passed"));
}

#[test]
fn agent_test_command_waits_for_approval_and_records_terminal_result_evidence() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.1", "backend.ollama");
    service.start(session.session_id());

    service.request_test_command(session.session_id(), "cargo test");
    let blocked = service.get(session.session_id()).unwrap();

    assert_eq!(blocked.owner(), SessionOwner::DesktopLab);
    assert_eq!(blocked.state(), SessionState::Blocked);
    assert_eq!(
        blocked.proposed_test_commands(),
        &["cargo test".to_string()]
    );
    assert!(blocked.terminal_evidence().is_empty());

    service.record_test_result(session.session_id(), "cargo test", "PASS", Some(0));
    let recorded = service.get(session.session_id()).unwrap();

    assert_eq!(recorded.terminal_evidence()[0].command(), "cargo test");
    assert_eq!(recorded.terminal_evidence()[0].output(), "PASS");
    assert_eq!(recorded.terminal_evidence()[0].exit_code(), Some(0));
}

#[test]
fn pause_resume_and_cancel_are_available_over_service_api() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace.1", "backend.ollama");

    service.pause(session.session_id(), "waiting");
    assert_eq!(
        service.get(session.session_id()).unwrap().state(),
        SessionState::Paused
    );
    service.resume(session.session_id());
    assert_eq!(
        service.get(session.session_id()).unwrap().state(),
        SessionState::Running
    );
    service.cancel(session.session_id(), "stop");
    assert_eq!(
        service.get(session.session_id()).unwrap().state(),
        SessionState::Cancelled
    );
}

#[test]
fn session_service_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/sessions.rs",
        include_str!("../src/sessions.rs"),
        320,
    )
    .expect("session service source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_recovery.rs",
        include_str!("../src/session_recovery.rs"),
        120,
    )
    .expect("session recovery source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_job_service.rs",
        include_str!("../src/session_job_service.rs"),
        120,
    )
    .expect("session job service source should stay below the line-count guard");
}
