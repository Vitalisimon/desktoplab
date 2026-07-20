use desktoplab_agent_session::{SessionEvent, SessionOwner};
use desktoplab_backends::{ExternalBackendHarness, ExternalBackendManifest, ExternalEvent};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use xtask::check_logical_line_limit;

#[test]
fn external_backend_cannot_own_session_state() {
    let harness = ExternalBackendHarness::new(ExternalBackendManifest::new(
        "backend.external",
        &["llm.chat"],
    ));

    let session = harness.create_session("session.external");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.external");
}

#[test]
fn streamed_external_events_normalize_into_desktoplab_events() {
    let harness = ExternalBackendHarness::new(ExternalBackendManifest::new(
        "backend.external",
        &["llm.chat"],
    ));

    let event = harness.normalize_event(ExternalEvent::text_delta("thinking"));

    assert_eq!(event, SessionEvent::planning_started("thinking"));
}

#[test]
fn missing_capability_blocks_external_backend_route() {
    let harness = ExternalBackendHarness::new(ExternalBackendManifest::new(
        "backend.external",
        &["llm.chat"],
    ));
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["tools.filesystem.write"]),
        vec![harness.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"missing_capability:tools.filesystem.write".to_string())
    );
}

#[test]
fn external_backend_harness_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/external_harness.rs",
        include_str!("../src/external_harness.rs"),
        250,
    )
    .expect("external backend harness source should stay below the initial line-count guard");
}
