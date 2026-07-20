use desktoplab_backend_services::{
    BackendEventScope, BackendEventStreamService, EventReplayRequest,
};
use xtask::check_logical_line_limit;

#[test]
fn event_stream_replays_after_last_event_id() {
    let mut stream = BackendEventStreamService::default();
    stream.publish(BackendEventScope::Job, "queued");
    stream.publish(BackendEventScope::Job, "started");
    stream.publish(BackendEventScope::Job, "completed");

    let replay = stream.replay(EventReplayRequest::new().after_sequence(1));

    assert_eq!(replay.sequences(), vec![2, 3]);
    assert_eq!(replay.payloads(), vec!["started", "completed"]);
}

#[test]
fn stream_payloads_are_redacted_before_delivery() {
    let mut stream = BackendEventStreamService::default();
    stream.publish(
        BackendEventScope::Approval,
        "provider token=sk-live-secret should never appear",
    );

    let replay = stream.replay(EventReplayRequest::new());

    assert_eq!(
        replay.payloads(),
        vec!["provider token=[REDACTED] should never appear"]
    );
}

#[test]
fn heartbeat_does_not_advance_persisted_sequence() {
    let mut stream = BackendEventStreamService::default();
    stream.publish(BackendEventScope::Session, "created");
    let heartbeat = stream.heartbeat();
    stream.publish(BackendEventScope::Session, "running");

    assert!(heartbeat.sequence().is_none());
    assert_eq!(
        stream.replay(EventReplayRequest::new()).sequences(),
        vec![1, 2]
    );
}

#[test]
fn filtered_streams_only_include_requested_scope() {
    let mut stream = BackendEventStreamService::default();
    stream.publish(BackendEventScope::Job, "job queued");
    stream.publish(BackendEventScope::Session, "session blocked");
    stream.publish(BackendEventScope::Setup, "plan accepted");

    let replay = stream.replay(EventReplayRequest::new().scope(BackendEventScope::Session));

    assert_eq!(replay.scopes(), vec![BackendEventScope::Session]);
    assert_eq!(replay.payloads(), vec!["session blocked"]);
}

#[test]
fn terminal_events_are_replayed_with_terminal_scope_and_redacted_payloads() {
    let mut stream = BackendEventStreamService::default();
    stream.publish_terminal_started("terminal.local", "workspace.desktoplab", "npm test");
    stream.publish_terminal_output("terminal.local", "stdout", "token=secret PASS", false);
    stream.publish_terminal_completed("terminal.local", Some(0));

    let replay = stream.replay(EventReplayRequest::new().scope(BackendEventScope::Terminal));

    assert_eq!(
        replay.scopes(),
        vec![
            BackendEventScope::Terminal,
            BackendEventScope::Terminal,
            BackendEventScope::Terminal
        ]
    );
    assert!(replay.payloads()[0].contains(r#""kind":"terminal.started""#));
    assert!(replay.payloads()[1].contains("token=[REDACTED]"));
    assert!(!replay.payloads()[1].contains("token=secret"));
    assert!(replay.payloads()[2].contains(r#""kind":"terminal.completed""#));
}

#[test]
fn runtime_install_progress_events_are_job_scoped_and_redacted() {
    let mut stream = BackendEventStreamService::default();
    stream.publish_runtime_install_progress(
        "job.runtime.install",
        "running",
        64,
        "retryable",
        "downloading token=secret",
    );

    let replay = stream.replay(EventReplayRequest::new().scope(BackendEventScope::Job));

    assert_eq!(replay.scopes(), vec![BackendEventScope::Job]);
    assert!(replay.payloads()[0].contains(r#""kind":"runtime.install""#));
    assert!(replay.payloads()[0].contains(r#""progressPercent":64"#));
    assert!(replay.payloads()[0].contains(r#""retryClass":"retryable""#));
    assert!(replay.payloads()[0].contains("token=[REDACTED]"));
    assert!(!replay.payloads()[0].contains("token=secret"));
}

#[test]
fn model_download_progress_events_are_job_scoped_and_replayable() {
    let mut stream = BackendEventStreamService::default();
    stream.publish_model_download_progress(
        "job.model.download",
        "model.qwen-coder-7b",
        "running",
        37,
        "retryable",
        "pulling token=secret",
    );

    let replay = stream.replay(EventReplayRequest::new().scope(BackendEventScope::Job));

    assert_eq!(replay.scopes(), vec![BackendEventScope::Job]);
    assert!(replay.payloads()[0].contains(r#""kind":"model.download""#));
    assert!(replay.payloads()[0].contains(r#""modelId":"model.qwen-coder-7b""#));
    assert!(replay.payloads()[0].contains(r#""progressPercent":37"#));
    assert!(replay.payloads()[0].contains(r#""retryClass":"retryable""#));
    assert!(replay.payloads()[0].contains("token=[REDACTED]"));
    assert!(!replay.payloads()[0].contains("token=secret"));
}

#[test]
fn event_stream_buffer_discards_old_frames_when_limit_is_reached() {
    let mut stream = BackendEventStreamService::with_event_limit(2);
    stream.publish(BackendEventScope::Job, "first");
    stream.publish(BackendEventScope::Job, "second");
    stream.publish(BackendEventScope::Job, "third");

    let replay = stream.replay(EventReplayRequest::new());

    assert_eq!(replay.sequences(), vec![2, 3]);
    assert_eq!(replay.payloads(), vec!["second", "third"]);
    assert_eq!(replay.oldest_sequence(), Some(2));
    assert_eq!(replay.latest_sequence(), 3);
    assert!(replay.gap_detected());
}

#[test]
fn replay_reports_pagination_and_stream_reset_without_false_continuity() {
    let mut stream = BackendEventStreamService::default();
    stream.publish(BackendEventScope::Job, "first");
    stream.publish(BackendEventScope::Job, "second");
    let first = stream.replay(EventReplayRequest::new().max_events(1));
    assert!(first.has_more());
    assert_eq!(first.next_sequence(), 1);

    let reset = stream.replay(
        EventReplayRequest::new()
            .after_sequence(99)
            .expected_stream_id("stale-stream"),
    );
    assert!(reset.reset_required());
    assert_eq!(reset.sequences(), vec![1, 2]);
    assert!(!reset.stream_id().is_empty());
}

#[test]
fn event_stream_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/event_stream.rs",
        include_str!("../src/event_stream.rs"),
        260,
    )
    .expect("event stream api source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/event_stream/replay.rs",
        include_str!("../src/event_stream/replay.rs"),
        150,
    )
    .expect("event replay protocol should stay focused");
}
