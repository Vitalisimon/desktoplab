use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopState, IterativeLoopStatus, IterativeModelDecision,
    IterativeToolCall, IterativeToolExecutor, ToolObservation,
};
use serde_json::json;

use super::{BackendDecisionAdapter, completion_decision};

struct StaticExecutor(serde_json::Value);

impl IterativeToolExecutor for StaticExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::success(call, self.0.clone()))
    }
}

struct FailingExecutor;

impl IterativeToolExecutor for FailingExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::failure(call, "executor_failed"))
    }
}

fn state_after_successful_call(
    session_id: &str,
    call_json: &'static str,
    output: serde_json::Value,
) -> IterativeLoopState {
    let mut adapter =
        BackendDecisionAdapter::new("Collect evidence", move |_| Ok(call_json.to_string()));
    let mut state = IterativeLoopState::new(session_id);
    IterativeAgentLoop::default().advance(&mut state, &mut adapter, &mut StaticExecutor(output));
    state
}

fn run_sequence(
    session_id: &str,
    outputs: &[&str],
    executor: &mut impl IterativeToolExecutor,
) -> IterativeLoopState {
    let mut turn = 0;
    let mut adapter = BackendDecisionAdapter::new("Complete with evidence", move |_| {
        let output = outputs[turn].to_string();
        turn += 1;
        Ok(output)
    });
    let mut state = IterativeLoopState::new(session_id);
    IterativeAgentLoop::default().run(&mut state, &mut adapter, executor);
    state
}

#[test]
fn completion_after_tool_use_requires_cited_successful_evidence() {
    let state = state_after_successful_call(
        "session.missing-evidence",
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        json!({"path":"README.md","text":"DesktopLab"}),
    );

    assert_eq!(
        completion_decision(
            &state,
            &json!({"message":"README inspected.","outcome":"answered","evidenceCallIds":[]})
        ),
        Err("completion_evidence_required".to_string())
    );
}

#[test]
fn executed_outcome_requires_a_successful_executor_observation() {
    let state = IterativeLoopState::new("session.executed-without-evidence");

    assert_eq!(
        completion_decision(
            &state,
            &json!({"message":"Command executed.","outcome":"executed","evidenceCallIds":[]})
        ),
        Err("completion_execution_evidence_required".to_string())
    );
}

#[test]
fn failed_observation_cannot_be_cited_as_completion_evidence() {
    let mut adapter = BackendDecisionAdapter::new("Run tests", |_| {
        Ok(
            r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#
                .to_string(),
        )
    });
    let mut state = IterativeLoopState::new("session.failed-evidence");
    IterativeAgentLoop::default().advance(&mut state, &mut adapter, &mut FailingExecutor);

    assert_eq!(
        completion_decision(
            &state,
            &json!({"message":"npm test passed.","outcome":"verified","evidenceCallIds":["test-1"]})
        ),
        Err("completion_evidence_invalid:test-1".to_string())
    );
}

#[test]
fn verified_outcome_requires_explicit_passing_test_evidence() {
    let state = state_after_successful_call(
        "session.false-verification",
        r#"{"id":"terminal-1","tool":"desktoplab.run_terminal","arguments":{"command":"pwd"}}"#,
        json!({"command":"pwd","status":"exited","exitCode":0,"passed":false}),
    );
    assert_eq!(
        completion_decision(
            &state,
            &json!({"message":"pwd verified.","outcome":"verified","evidenceCallIds":["terminal-1"]})
        ),
        Err("completion_test_evidence_required".to_string())
    );

    let state = state_after_successful_call(
        "session.real-verification",
        r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#,
        json!({"command":"npm test","status":"exited","exitCode":0,"passed":true}),
    );
    assert_eq!(
        completion_decision(
            &state,
            &json!({"message":"npm test passed.","outcome":"verified","evidenceCallIds":["test-1"]})
        ),
        Ok(IterativeModelDecision::final_response("npm test passed."))
    );
}

#[test]
fn changed_outcome_requires_real_change_or_dirty_git_report() {
    let false_change = [
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Changed README.","outcome":"changed","evidenceCallIds":["read-1"]}}"#,
    ];
    let state = run_sequence(
        "session.false-change",
        &false_change,
        &mut StaticExecutor(json!({"path":"README.md","text":"DesktopLab"})),
    );
    assert_eq!(state.status(), IterativeLoopStatus::Failed);

    let clean_git = [
        r#"{"id":"status-1","tool":"desktoplab.git_status","arguments":{}}"#,
        r#"{"id":"diff-1","tool":"desktoplab.git_diff","arguments":{}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"The workspace changed.","outcome":"changed","evidenceCallIds":["status-1","diff-1"]}}"#,
    ];
    let state = run_sequence(
        "session.clean-git-change",
        &clean_git,
        &mut StaticExecutor(json!({"entries":[],"diff":""})),
    );
    assert_eq!(state.status(), IterativeLoopStatus::Failed);
}

#[test]
fn changed_outcome_accepts_dirty_git_inspection_with_read_only_context() {
    let outputs = [
        r#"{"id":"status-1","tool":"desktoplab.git_status","arguments":{}}"#,
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"calculator.js"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"calculator.js is modified and now adds values.","outcome":"changed","evidenceCallIds":["status-1","read-1"]}}"#,
    ];
    let state = run_sequence(
        "session.git-change-with-read",
        &outputs,
        &mut StaticExecutor(json!({
            "entries":[" M calculator.js"],
            "path":"calculator.js",
            "content":"export const add = (left, right) => left + right;"
        })),
    );

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
}

#[test]
fn changed_outcome_accepts_complete_git_status_and_diff_report() {
    let outputs = [
        r#"{"id":"status-1","tool":"desktoplab.git_status","arguments":{}}"#,
        r#"{"id":"diff-1","tool":"desktoplab.git_diff","arguments":{}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"calculator.js and notes.md are modified; agent-note.md is untracked. calculator.js now adds values and notes.md updates beta.","outcome":"changed","evidenceCallIds":["status-1","diff-1"]}}"#,
    ];
    let state = run_sequence(
        "session.git-change-report",
        &outputs,
        &mut StaticExecutor(json!({
            "entries":[" M calculator.js"," M notes.md","?? agent-note.md"],
            "diff":"calculator.js adds values and notes.md updates beta"
        })),
    );

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
}

#[test]
fn changed_outcome_requires_changed_true_for_file_mutations() {
    for changed in [false, true] {
        let outputs = [
            r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"README.md","content":"new"}}"#,
            r#"{"tool":"desktoplab.complete","arguments":{"message":"Changed README.","outcome":"changed","evidenceCallIds":["write-1"]}}"#,
        ];
        let state = run_sequence(
            if changed {
                "session.changed-write"
            } else {
                "session.unchanged-write"
            },
            &outputs,
            &mut StaticExecutor(json!({"path":"README.md","changed":changed})),
        );
        assert_eq!(state.status(), IterativeLoopStatus::Failed);
    }
}

#[test]
fn completion_outcome_tests_stay_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_model_adapter/completion_tests.rs",
        include_str!("completion_tests.rs"),
        380,
    )
    .expect("completion outcome tests should stay focused");
}
