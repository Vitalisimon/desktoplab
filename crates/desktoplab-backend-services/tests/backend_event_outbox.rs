use desktoplab_backend_services::{
    BackendEventScope, BackendEventStreamService, EventReplayRequest,
};
use serde_json::json;

#[test]
fn typed_event_outbox_round_trips_sequence_scope_and_redacted_payload() {
    let mut events = BackendEventStreamService::default();
    events.publish_json(
        BackendEventScope::Session,
        json!({
            "kind":"agent.delta",
            "message":"token=secret",
            "nested":{"quote":"a \"real\" value"}
        }),
    );
    let restored = BackendEventStreamService::from_json(&events.to_json()).unwrap();
    let replay = restored.replay(EventReplayRequest::new());

    assert_eq!(replay.sequences(), vec![1]);
    assert_eq!(replay.scopes(), vec![BackendEventScope::Session]);
    let payload: serde_json::Value = serde_json::from_str(replay.payloads()[0]).unwrap();
    assert_eq!(payload["message"], "token=[REDACTED]");
    assert_eq!(payload["nested"]["quote"], "a \"real\" value");
}

#[test]
fn restored_outbox_continues_monotonic_sequence_and_reapplies_bound() {
    let mut events = BackendEventStreamService::with_event_limit(2);
    events.publish(BackendEventScope::Job, "first");
    events.publish(BackendEventScope::Job, "second");
    let mut restored = BackendEventStreamService::from_json(&events.to_json()).unwrap();
    restored.publish(BackendEventScope::Job, "third");

    let replay = restored.replay(EventReplayRequest::new());
    assert_eq!(replay.sequences(), vec![2, 3]);
    assert_eq!(replay.payloads(), vec!["second", "third"]);
}

#[test]
fn event_outbox_sources_stay_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/src/event_stream/payload.rs",
        include_str!("../src/event_stream/payload.rs"),
        80,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/tests/backend_event_outbox.rs",
        include_str!("backend_event_outbox.rs"),
        100,
    )
    .unwrap();
}
