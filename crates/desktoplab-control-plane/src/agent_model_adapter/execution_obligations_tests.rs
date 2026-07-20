use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopState, IterativeModelAdapter, IterativeModelDecision,
    IterativeToolCall, IterativeToolExecutor, ToolObservation,
};
use serde_json::json;

use super::{completion_decision, BackendDecisionAdapter};

struct StaticExecutor(serde_json::Value);

impl IterativeToolExecutor for StaticExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::success(call, self.0.clone()))
    }
}

#[test]
fn patch_requires_a_successful_read_of_the_same_target() {
    let state = IterativeLoopState::new("session.patch-before-read");
    let mut patch = adapter(
        r#"{"id":"patch-1","tool":"desktoplab.patch_file","arguments":{"path":"notes.md","expected":"old","replacement":"new"}}"#,
    );
    assert_eq!(
        patch.decide(&state),
        Err("patch_requires_prior_read".to_string())
    );

    let mut state = state;
    advance(
        &mut state,
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
        json!({"path":"notes.md","text":"old"}),
    );
    assert!(matches!(
        patch.decide(&state),
        Ok(IterativeModelDecision::ToolCall(_))
    ));

    advance(
        &mut state,
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"notes.md","content":"newer"}}"#,
        json!({"path":"notes.md","changed":true}),
    );
    assert_eq!(
        patch.decide(&state),
        Err("patch_requires_prior_read".to_string())
    );
    advance(
        &mut state,
        r#"{"id":"read-2","tool":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
        json!({"path":"notes.md","text":"newer"}),
    );
    assert!(matches!(
        patch.decide(&state),
        Ok(IterativeModelDecision::ToolCall(_))
    ));
}

#[test]
fn changed_file_completion_requires_cited_post_change_inspection() {
    let mut state = IterativeLoopState::new("session.change-verification");
    advance(
        &mut state,
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"notes.md","content":"release proof"}}"#,
        json!({"path":"notes.md","changed":true}),
    );
    let mut completion = json!({"message":"notes.md contains release proof.","outcome":"changed","evidenceCallIds":["write-1"]});
    assert_eq!(
        completion_decision(&state, &completion),
        Err("completion_post_change_inspection_required".to_string())
    );

    advance(
        &mut state,
        r#"{"id":"read-2","tool":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
        json!({"path":"notes.md","text":"release proof"}),
    );
    completion["evidenceCallIds"] = json!(["write-1", "read-2"]);
    assert_eq!(
        completion_decision(&state, &completion),
        Ok(IterativeModelDecision::final_response(
            "notes.md contains release proof."
        ))
    );
}

#[test]
fn cited_patch_diff_is_post_change_inspection_evidence() {
    let mut state = IterativeLoopState::new("session.patch-verification");
    advance(
        &mut state,
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
        json!({"path":"notes.md","text":"old"}),
    );
    advance(
        &mut state,
        r#"{"id":"patch-1","tool":"desktoplab.patch_file","arguments":{"path":"notes.md","expected":"old","replacement":"new"}}"#,
        json!({"path":"notes.md","changed":true,"diff":"-old\n+new\n"}),
    );
    let completion = json!({"message":"notes.md now contains new.","outcome":"changed","evidenceCallIds":["patch-1"]});

    assert_eq!(
        completion_decision(&state, &completion),
        Ok(IterativeModelDecision::final_response(
            "notes.md now contains new."
        ))
    );
}

#[test]
fn execution_obligation_modules_stay_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_execution_obligations.rs",
        include_str!("../agent_execution_obligations.rs"),
        100,
    )
    .expect("execution obligations should stay focused");
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_model_adapter/execution_obligations_tests.rs",
        include_str!("execution_obligations_tests.rs"),
        140,
    )
    .expect("execution obligation tests should stay focused");
}

fn adapter(
    output: &'static str,
) -> BackendDecisionAdapter<
    impl FnMut(Vec<desktoplab_backends::BackendMessage>) -> Result<String, String>,
> {
    BackendDecisionAdapter::new("Work on notes.md", move |_| Ok(output.to_string()))
}

fn advance(state: &mut IterativeLoopState, output: &'static str, observation: serde_json::Value) {
    IterativeAgentLoop::default().advance(
        state,
        &mut adapter(output),
        &mut StaticExecutor(observation),
    );
}
