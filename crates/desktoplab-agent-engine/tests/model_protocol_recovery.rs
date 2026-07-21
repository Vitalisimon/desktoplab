use desktoplab_agent_engine::{IterativeLoopEvent, IterativeLoopState};

#[test]
fn protocol_retry_is_bounded_and_survives_serialization() {
    let mut state = IterativeLoopState::new("session.protocol");

    assert!(state.request_model_protocol_retry("unknown_tool:read"));
    assert!(state.request_model_protocol_retry("unknown_tool:read"));
    assert!(!state.request_model_protocol_retry("unknown_tool:list"));
    let restored = IterativeLoopState::from_json(&state.to_json().unwrap()).unwrap();

    assert_eq!(
        restored.model_protocol_recovery(),
        Some("unknown_tool:read")
    );
    assert_eq!(
        restored
            .events()
            .iter()
            .filter(|event| matches!(event, IterativeLoopEvent::ModelProtocolRetry { .. }))
            .count(),
        2
    );
    assert!(restored.events().iter().any(|event| matches!(
        event,
        IterativeLoopEvent::ModelProtocolRetry { ordinal: 2, reason }
            if reason == "unknown_tool:read"
    )));
}

#[test]
fn canonical_response_opens_one_fresh_bounded_retry_window() {
    let mut state = IterativeLoopState::new("session.protocol");

    assert!(state.request_model_protocol_retry("first"));
    state.clear_model_protocol_recovery();

    assert_eq!(state.model_protocol_recovery(), None);
    assert!(state.request_model_protocol_retry("second"));
}
