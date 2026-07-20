use desktoplab_agent_engine::{
    AgentFailureKind, FailureObservation, RetryAttempt, RetryDecision, RetryPolicy,
};
use xtask::check_logical_line_limit;

#[test]
fn failed_validation_allows_one_patch_and_rerun_then_stops_truthfully() {
    let policy = RetryPolicy::new(1);
    let failure = FailureObservation::test_failed("cargo test failed: assertion mismatch");

    let first = policy.evaluate(&[], &failure);

    assert_eq!(first.decision(), RetryDecision::Retry);
    assert_eq!(first.reason(), "retryable_validation_failure");

    let attempts = vec![
        RetryAttempt::from_observation(failure.clone())
            .with_patch_summary("patched expected assertion")
            .with_rerun_summary("cargo test failed again"),
    ];
    let final_result = policy.evaluate(
        &attempts,
        &FailureObservation::test_failed("cargo test failed again"),
    );

    assert_eq!(final_result.decision(), RetryDecision::Stop);
    assert_eq!(final_result.reason(), "max_retry_count_reached");
    assert!(final_result.truthful_summary().contains("still failing"));
}

#[test]
fn retry_policy_distinguishes_non_retryable_failures() {
    let policy = RetryPolicy::new(2);

    for kind in [
        AgentFailureKind::ModelRefusal,
        AgentFailureKind::PolicyDenial,
    ] {
        let result = policy.evaluate(&[], &FailureObservation::new(kind, "blocked"));
        assert_eq!(result.decision(), RetryDecision::Stop);
    }

    for kind in [
        AgentFailureKind::ToolFailure,
        AgentFailureKind::TestFailure,
        AgentFailureKind::Timeout,
    ] {
        let result = policy.evaluate(&[], &FailureObservation::new(kind, "retryable"));
        assert_eq!(result.decision(), RetryDecision::Retry);
    }
}

#[test]
fn failure_retry_loop_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/tests/failure_retry_loop.rs",
        include_str!("failure_retry_loop.rs"),
        110,
    )
    .expect("failure retry loop test should stay focused");
}
