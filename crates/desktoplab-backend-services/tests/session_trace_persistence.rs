use desktoplab_agent_session::SessionEvent;
use desktoplab_backend_services::{SessionService, SessionServiceStore};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn production_session_trace_survives_restart_without_raw_private_content() {
    let fixture = TempDir::new().expect("fixture should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let mut service = SessionService::new(
        SessionServiceStore::with_storage(migrated_store(&db_path)).expect("store opens"),
    );
    let session = service.create_session("workspace.desktoplab", "backend.ollama");
    service.plan(
        session.session_id(),
        "Read /Users/private/repo and use token=sk-private-secret",
    );
    service.append_events(
        session.session_id(),
        &[SessionEvent::tool_decision_recorded(
            "state=observed source=filesystem.write tool=filesystem.write:/Users/private/repo/secret.md approval_mode=require_approval",
        )],
    );
    service.record_test_result(
        session.session_id(),
        "cat /Users/private/repo/.env",
        "token=sk-terminal-secret",
        Some(0),
    );
    service.complete(session.session_id(), "Wrote /Users/private/repo/secret.md");

    let restarted = SessionService::new(
        SessionServiceStore::with_storage(migrated_store(&db_path)).expect("store reopens"),
    );
    let trace = restarted
        .trace(session.session_id())
        .expect("trace should survive restart");
    let jsonl = trace.to_jsonl().expect("trace should serialize");

    assert_eq!(trace.schema_version(), 1);
    assert_eq!(trace.events().len(), 5);
    assert_eq!(trace.events()[0].event_id(), "session.1:trace:1");
    assert_eq!(trace.events()[4].sequence(), 5);
    assert_eq!(trace.events()[3].kind(), "terminal_observed");
    assert_eq!(trace.events()[3].success(), Some(true));
    assert!(jsonl.contains("desktoplab.write_file"));
    assert!(!jsonl.contains("/Users/private"));
    assert!(!jsonl.contains("sk-private-secret"));
    assert!(!jsonl.contains("sk-terminal-secret"));
    assert!(!jsonl.contains("cat "));
}

#[test]
fn session_trace_source_stays_bounded() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_trace.rs",
        include_str!("../src/session_trace.rs"),
        300,
    )
    .expect("production session trace should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/session_trace_metadata.rs",
        include_str!("../src/session_trace_metadata.rs"),
        260,
    )
    .expect("session event metadata mapping should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/session_trace_persistence.rs",
        include_str!("session_trace_persistence.rs"),
        110,
    )
    .expect("session trace persistence test should stay focused");
}

#[test]
fn trace_rejects_oversized_backend_identifiers() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let oversized = "private".repeat(30);
    let session = service.create_session("workspace.desktoplab", &oversized);
    let jsonl = service
        .trace(session.session_id())
        .expect("trace should exist")
        .to_jsonl()
        .expect("trace should serialize");

    assert!(!jsonl.contains(&oversized));
    assert!(jsonl.contains(r#""source":"backend""#));
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).expect("store should open");
    store.apply_migrations().expect("migrations should apply");
    store
}
