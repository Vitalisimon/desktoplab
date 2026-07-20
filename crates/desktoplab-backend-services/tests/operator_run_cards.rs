use desktoplab_backend_services::{RunCardError, RunCardService, RunCardState, TakeoverRequest};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;

#[test]
fn run_cards_persist_heartbeat_stale_takeover_and_archive() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("state.sqlite");
    {
        let store = migrated_store(&database);
        let service = RunCardService::new(&store);
        let created = service
            .create("run.1", "agent.alpha", "n95-linux", "scheduled_review", 100)
            .unwrap();
        assert_eq!(created.state, RunCardState::Scheduled);
        assert_eq!(created.attempt, 1);
        assert_eq!(
            service.heartbeat("run.1", "other", 120),
            Err(RunCardError::OwnerMismatch)
        );
        let running = service.heartbeat("run.1", "agent.alpha", 120).unwrap();
        assert_eq!(running.state, RunCardState::Running);
    }

    let store = migrated_store(&database);
    let service = RunCardService::new(&store);
    let stale = service.reconcile_stale("run.1", 500, 100).unwrap();
    assert_eq!(stale.state, RunCardState::Stale);
    let watched = service.watch("run.1").unwrap();
    assert_eq!(watched.events, stale.events);

    assert_eq!(
        service.takeover(
            "run.1",
            TakeoverRequest {
                new_owner_id: "operator.one",
                has_takeover_capability: true,
                policy_approved: false,
                at_ms: 510,
            }
        ),
        Err(RunCardError::TakeoverDenied)
    );
    let taken = service
        .takeover(
            "run.1",
            TakeoverRequest {
                new_owner_id: "operator.one",
                has_takeover_capability: true,
                policy_approved: true,
                at_ms: 520,
            },
        )
        .unwrap();
    assert_eq!(taken.owner_id, "operator.one");
    assert_eq!(taken.attempt, 2);

    let completed = service
        .complete(
            "run.1",
            "operator.one",
            "Reviewed and patched.",
            &["evidence://bundle/run.1".to_string()],
            700,
        )
        .unwrap();
    assert_eq!(completed.state, RunCardState::Completed);
    assert_eq!(
        completed.transcript_summary.as_deref(),
        Some("Reviewed and patched.")
    );
    assert_eq!(
        service.heartbeat("run.1", "operator.one", 800),
        Err(RunCardError::Terminal)
    );
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).unwrap();
    store.apply_migrations().unwrap();
    store
}

#[test]
fn run_card_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/src/run_cards.rs",
        include_str!("../src/run_cards.rs"),
        300,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/operator_run_cards.rs",
        include_str!("operator_run_cards.rs"),
        140,
    )
    .unwrap();
}
