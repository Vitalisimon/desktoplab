use std::sync::atomic::{AtomicBool, Ordering};

use super::retry_inference_timeout;

#[test]
fn one_timeout_retries_the_model_call_once() {
    let cancellation = AtomicBool::new(false);
    let mut calls = 0;
    let output = retry_inference_timeout(&cancellation, || {
        calls += 1;
        if calls == 1 {
            Err("ollama_request_timeout".to_string())
        } else {
            Ok("response".to_string())
        }
    });
    assert_eq!(output.as_deref(), Ok("response"));
    assert_eq!(calls, 2);
}

#[test]
fn repeated_timeout_stops_at_two_attempts() {
    let cancellation = AtomicBool::new(false);
    let mut calls = 0;
    let result = retry_inference_timeout(&cancellation, || {
        calls += 1;
        Err("ollama_request_timeout".to_string())
    });
    assert_eq!(result.unwrap_err(), "ollama_request_timeout");
    assert_eq!(calls, 2);
}

#[test]
fn non_timeout_is_never_retried() {
    let cancellation = AtomicBool::new(false);
    let mut calls = 0;
    let result = retry_inference_timeout(&cancellation, || {
        calls += 1;
        Err("ollama_request_failed:offline".to_string())
    });
    assert_eq!(result.unwrap_err(), "ollama_request_failed:offline");
    assert_eq!(calls, 1);
}

#[test]
fn a_failed_stream_after_output_is_never_replayed() {
    let cancellation = AtomicBool::new(false);
    let mut calls = 0;
    let result = retry_inference_timeout(&cancellation, || {
        calls += 1;
        Err("ollama_stream_read_failed:timed out".to_string())
    });
    assert_eq!(result.unwrap_err(), "ollama_stream_read_failed:timed out");
    assert_eq!(calls, 1);
}

#[test]
fn cancellation_prevents_the_first_or_second_attempt() {
    let cancellation = AtomicBool::new(true);
    let mut calls = 0;
    let before = retry_inference_timeout(&cancellation, || {
        calls += 1;
        Ok("response".to_string())
    });
    assert_eq!(before.unwrap_err(), "agent_cancelled");
    assert_eq!(calls, 0);
    cancellation.store(false, Ordering::SeqCst);
    let during = retry_inference_timeout(&cancellation, || {
        calls += 1;
        cancellation.store(true, Ordering::SeqCst);
        Err("ollama_request_timeout".to_string())
    });
    assert_eq!(during.unwrap_err(), "agent_cancelled");
    assert_eq!(calls, 1);
}

#[test]
fn cancellation_discards_a_late_success_before_any_tool_decision() {
    let cancellation = AtomicBool::new(false);
    let result = retry_inference_timeout(&cancellation, || {
        cancellation.store(true, Ordering::SeqCst);
        Ok("late tool response".to_string())
    });
    assert_eq!(result.unwrap_err(), "agent_cancelled");
}

#[test]
fn retry_sources_stay_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_ollama_timeout_retry.rs",
        include_str!("agent_ollama_timeout_retry.rs"),
        45,
    )
    .expect("timeout recovery should remain focused");
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_ollama_timeout_retry_tests.rs",
        include_str!("agent_ollama_timeout_retry_tests.rs"),
        120,
    )
    .expect("timeout recovery tests should remain focused");
}
