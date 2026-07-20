use desktoplab_agent_engine::{IterativeToolCall, ToolObservation};
use serde_json::json;

use super::readable_observation;

#[test]
fn failed_test_output_is_human_readable_without_internal_codes() {
    let call = IterativeToolCall::new(
        "call.1",
        "desktoplab.run_tests",
        json!({ "command": "node test.js" }),
    );
    let observation = ToolObservation::failure_with_output(
        &call,
        json!({
            "command": "node test.js",
            "status": "exited",
            "exitCode": 1,
            "stdout": "",
            "stderr": "assertion failed"
        }),
        "tests_failed:1",
    );

    let message = readable_observation(&observation);
    assert_eq!(
        message,
        "Test command `node test.js` failed (exit code 1).\nErrors:\nassertion failed"
    );
    assert!(!message.contains("tests_failed"));
    assert!(!message.contains("error="));
    assert!(!message.contains("status exited"));
}

#[test]
fn command_statuses_have_stable_human_copy() {
    let call = IterativeToolCall::new("call.2", "desktoplab.run_terminal", json!({ "command": "pwd" }));
    for (status, exit_code, expected) in [
        ("exited", Some(0), "completed successfully"),
        ("exited", Some(7), "finished with exit code 7"),
        ("timed_out", None, "timed out"),
        ("failed_to_spawn", None, "could not start"),
    ] {
        let observation = ToolObservation::success(&call, json!({
            "command": "pwd", "status": status, "exitCode": exit_code, "stdout": "", "stderr": ""
        }));
        assert!(readable_observation(&observation).contains(expected));
    }
}

#[test]
fn command_display_sources_stay_focused() {
    let logical = include_str!("agent_observation_display.rs")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert!(
        logical <= 180,
        "agent_observation_display.rs has {logical} logical lines"
    );
    assert!(
        include_str!("agent_observation_display_tests.rs")
            .lines()
            .count()
            <= 80
    );
}
