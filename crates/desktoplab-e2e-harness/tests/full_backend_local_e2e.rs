use desktoplab_agent_session::SessionState;
use desktoplab_backend_services::{
    ApprovalResolution, ApprovalService, ApprovalState, ApprovalStore, SessionService,
    SessionServiceStore,
};
use desktoplab_smoke_cli::{InProcessSmokeApi, SmokeCli, SmokeCommand};
use xtask::check_logical_line_limit;

#[test]
fn full_backend_local_happy_path_runs_without_frontend() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());
    assert_eq!(
        cli.run(SmokeCommand::Health).body(),
        r#"{"status":"healthy"}"#
    );
    assert!(
        cli.run(SmokeCommand::WorkspaceOpen("workspace.e2e".to_string()))
            .body()
            .contains("workspace.e2e")
    );
    assert!(cli.run(SmokeCommand::SetupPreview).is_json());

    let mut approvals = ApprovalService::new(ApprovalStore::default());
    let approval = approvals.request("session.e2e", "filesystem.write README.md");
    let resolved = approvals
        .resolve(approval.id(), ApprovalResolution::Approve)
        .expect("approval should resolve");
    assert_eq!(resolved.state(), ApprovalState::Approved);

    let store = SessionServiceStore::default();
    let mut sessions = SessionService::new(store.clone());
    let session = sessions.create_session("workspace.e2e", "backend.local");
    sessions.start(session.session_id());
    sessions.complete(session.session_id(), "diff captured; tests passed");

    let replayed = SessionService::new(store)
        .replay(session.session_id())
        .expect("session should replay after restart");
    assert_eq!(replayed.state(), SessionState::Completed);
    assert_eq!(replayed.summary(), Some("diff captured; tests passed"));
}

#[test]
fn full_backend_local_e2e_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/tests/full_backend_local_e2e.rs",
        include_str!("full_backend_local_e2e.rs"),
        160,
    )
    .expect("local e2e source should stay below the line-count guard");
}
