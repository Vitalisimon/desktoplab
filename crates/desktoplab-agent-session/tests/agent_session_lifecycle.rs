use desktoplab_agent_session::{
    AgentSession, CheckpointRef, SessionEvent, SessionOwner, SessionReplay, SessionState,
};
use xtask::check_logical_line_limit;

#[test]
fn desktoplab_owns_session_even_when_backend_executes() {
    let session = AgentSession::new("session.1", "backend.codex");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.codex");
}

#[test]
fn session_replay_reconstructs_current_state() {
    let events = vec![
        SessionEvent::created("session.1", "backend.local"),
        SessionEvent::planning_started("draft plan"),
        SessionEvent::execution_started(),
        SessionEvent::checkpoint_created(CheckpointRef::new("checkpoint.1")),
        SessionEvent::completed("tests passed"),
    ];

    let session = SessionReplay::replay(events).expect("events should replay");

    assert_eq!(session.state(), SessionState::Completed);
    assert_eq!(session.summary(), Some("tests passed"));
    assert_eq!(session.checkpoints().len(), 1);
}

#[test]
fn blocked_failed_cancelled_and_completed_states_are_distinct() {
    assert_ne!(SessionState::Blocked, SessionState::Failed);
    assert_ne!(SessionState::Failed, SessionState::Cancelled);
    assert_ne!(SessionState::Cancelled, SessionState::Completed);
}

#[test]
fn pause_resume_and_cancellation_are_explicit_events() {
    let mut session = AgentSession::new("session.2", "backend.local");

    session.apply(SessionEvent::paused("waiting for user"));
    assert_eq!(session.state(), SessionState::Paused);

    session.apply(SessionEvent::resumed());
    assert_eq!(session.state(), SessionState::Running);

    session.apply(SessionEvent::cancelled("user cancelled"));
    assert_eq!(session.state(), SessionState::Cancelled);
}

#[test]
fn leaving_blocked_state_clears_the_stale_blocked_reason() {
    let mut session = AgentSession::new("session.blocked", "backend.ollama");

    session.apply(SessionEvent::blocked("clarification_required:file_target"));
    assert_eq!(
        session.blocked_reason(),
        Some("clarification_required:file_target")
    );

    session.apply(SessionEvent::planning_started(
        "Continue with repository evidence",
    ));
    assert_eq!(session.state(), SessionState::Planning);
    assert_eq!(session.blocked_reason(), None);

    session.apply(SessionEvent::completed("agent loop completed"));
    assert_eq!(session.state(), SessionState::Completed);
    assert_eq!(session.blocked_reason(), None);
}

#[test]
fn background_job_tracks_real_session_lifecycle() {
    let mut session = AgentSession::new("session.job", "backend.ollama");
    session.apply(SessionEvent::job_started(
        "agent-job.session.job",
        "1",
        true,
    ));
    session.apply(SessionEvent::blocked("waiting for approval"));
    assert_eq!(session.job().unwrap().state(), "blocked");
    assert!(!session.job().unwrap().cancellable());

    session.apply(SessionEvent::resumed());
    assert_eq!(session.job().unwrap().state(), "running");
    assert!(session.job().unwrap().cancellable());

    session.apply(SessionEvent::completed("done"));
    assert_eq!(session.job().unwrap().state(), "completed");
    assert!(!session.job().unwrap().cancellable());
}

#[test]
fn agent_session_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-agent-session/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-session/src/session.rs",
            include_str!("../src/session.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-session/src/event.rs",
            include_str!("../src/event.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-session/src/job.rs",
            include_str!("../src/job.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-session/src/terminal_evidence.rs",
            include_str!("../src/terminal_evidence.rs"),
            250,
        ),
        (
            "crates/desktoplab-agent-session/src/replay.rs",
            include_str!("../src/replay.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("agent session source should stay below the initial line-count guard");
    }
}
