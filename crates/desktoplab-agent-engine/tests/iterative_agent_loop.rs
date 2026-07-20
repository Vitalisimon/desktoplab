use std::collections::VecDeque;
use std::time::Duration;

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopLimits, IterativeLoopState, IterativeLoopStatus,
    IterativeModelAdapter, IterativeModelDecision, IterativeToolCall, IterativeToolExecutor,
    ToolObservation,
};
use serde_json::json;
use xtask::check_logical_line_limit;

struct ScriptedModel {
    decisions: VecDeque<Result<IterativeModelDecision, String>>,
}

impl ScriptedModel {
    fn new(decisions: impl IntoIterator<Item = IterativeModelDecision>) -> Self {
        Self {
            decisions: decisions.into_iter().map(Ok).collect(),
        }
    }
}

impl IterativeModelAdapter for ScriptedModel {
    fn decide(&mut self, _state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        self.decisions
            .pop_front()
            .unwrap_or_else(|| Err("script exhausted".to_string()))
    }
}

#[derive(Default)]
struct RecordingExecutor {
    calls: Vec<String>,
    fail_with: Option<String>,
}

impl IterativeToolExecutor for RecordingExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        self.calls.push(call.id().to_string());
        if let Some(error) = &self.fail_with {
            return Err(error.clone());
        }
        Ok(ToolObservation::success(
            call,
            json!({ "result": format!("{} complete", call.name()) }),
        ))
    }
}

fn tool(id: &str, name: &str) -> IterativeModelDecision {
    IterativeModelDecision::tool_call(IterativeToolCall::new(id, name, json!({ "path": "." })))
}

#[test]
fn loops_across_three_model_turns_before_completing() {
    let mut model = ScriptedModel::new([
        tool("call-1", "workspace.list"),
        tool("call-2", "filesystem.read"),
        IterativeModelDecision::final_response("The repository instructions are loaded."),
    ]);
    let mut executor = RecordingExecutor::default();
    let mut state = IterativeLoopState::new("session-1");

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert_eq!(state.model_turns(), 3);
    assert_eq!(state.tool_calls(), 2);
    assert_eq!(executor.calls, ["call-1", "call-2"]);
    assert_eq!(
        state.final_response(),
        Some("The repository instructions are loaded.")
    );
    assert!(
        state
            .events()
            .last()
            .is_some_and(|event| event.is_completed())
    );
    assert!(
        state.events()[..state.events().len() - 1]
            .iter()
            .all(|event| !event.is_completed())
    );
}

#[test]
fn persisted_observation_resumes_without_reexecuting_completed_tool() {
    let mut first_model = ScriptedModel::new([tool("call-1", "filesystem.read")]);
    let mut executor = RecordingExecutor::default();
    let mut state = IterativeLoopState::new("session-restart");
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.advance(&mut state, &mut first_model, &mut executor);
    assert_eq!(executor.calls, ["call-1"]);
    assert_eq!(state.status(), IterativeLoopStatus::Running);

    let persisted = state.to_json().expect("state must serialize");
    let mut restored = IterativeLoopState::from_json(&persisted).expect("state must restore");
    let mut resumed_model = ScriptedModel::new([IterativeModelDecision::final_response(
        "README inspected successfully.",
    )]);

    agent_loop.run(&mut restored, &mut resumed_model, &mut executor);

    assert_eq!(restored.status(), IterativeLoopStatus::Completed);
    assert_eq!(executor.calls, ["call-1"]);
    assert_eq!(restored.observations().len(), 1);
    assert_eq!(restored.model_turns(), 2);
}

#[test]
fn replayed_tool_call_id_is_rejected_without_reexecution() {
    let mut first_model = ScriptedModel::new([tool("stable-call", "filesystem.read")]);
    let mut replay_model = ScriptedModel::new([tool("stable-call", "filesystem.read")]);
    let mut executor = RecordingExecutor::default();
    let mut state = IterativeLoopState::new("session-replay");
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.advance(&mut state, &mut first_model, &mut executor);
    agent_loop.advance(&mut state, &mut replay_model, &mut executor);

    assert_eq!(executor.calls, ["stable-call"]);
    assert_eq!(state.status(), IterativeLoopStatus::Failed);
    assert_eq!(state.stop_reason_code(), Some("duplicate_tool_call"));
}

#[test]
fn repeated_identical_tool_failures_exhaust_the_loop() {
    let mut model = ScriptedModel::new([
        tool("call-1", "terminal.run"),
        tool("call-2", "terminal.run"),
        tool("call-3", "terminal.run"),
        IterativeModelDecision::final_response("must not complete"),
    ]);
    let mut executor = RecordingExecutor {
        fail_with: Some("command failed".to_string()),
        ..RecordingExecutor::default()
    };
    let mut state = IterativeLoopState::new("session-failure");
    let limits = IterativeLoopLimits::new(8, 8, Duration::from_secs(5), 3);

    IterativeAgentLoop::new(limits).run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Exhausted);
    assert_eq!(state.stop_reason_code(), Some("repeated_tool_failure"));
    assert!(state.final_response().is_none());
}

#[test]
fn model_block_failure_and_cancellation_have_distinct_states() {
    let mut blocked_model = ScriptedModel::new([IterativeModelDecision::blocked("policy")]);
    let mut failed_model = ScriptedModel {
        decisions: [Err("provider unavailable".to_string())].into(),
    };
    let mut executor = RecordingExecutor::default();
    let mut blocked = IterativeLoopState::new("blocked");
    let mut failed = IterativeLoopState::new("failed");
    let mut cancelled = IterativeLoopState::new("cancelled");
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.run(&mut blocked, &mut blocked_model, &mut executor);
    agent_loop.run(&mut failed, &mut failed_model, &mut executor);
    cancelled.cancel("user_cancelled");

    assert_eq!(blocked.status(), IterativeLoopStatus::Blocked);
    assert_eq!(failed.status(), IterativeLoopStatus::Failed);
    assert_eq!(cancelled.status(), IterativeLoopStatus::Cancelled);
}

#[test]
fn turn_tool_and_duration_limits_exhaust_without_false_completion() {
    let mut executor = RecordingExecutor::default();
    let mut turn_state = IterativeLoopState::new("turn-limit");
    let mut tool_state = IterativeLoopState::new("tool-limit");
    let mut duration_state = IterativeLoopState::new("duration-limit");
    let mut turn_model = ScriptedModel::new([IterativeModelDecision::final_response("late")]);
    let mut tool_model = ScriptedModel::new([tool("call-1", "filesystem.read")]);
    let mut duration_model = ScriptedModel::new([IterativeModelDecision::final_response("late")]);

    IterativeAgentLoop::new(IterativeLoopLimits::new(0, 4, Duration::from_secs(5), 3)).run(
        &mut turn_state,
        &mut turn_model,
        &mut executor,
    );
    IterativeAgentLoop::new(IterativeLoopLimits::new(4, 0, Duration::from_secs(5), 3)).run(
        &mut tool_state,
        &mut tool_model,
        &mut executor,
    );
    IterativeAgentLoop::new(IterativeLoopLimits::new(4, 4, Duration::ZERO, 3)).run(
        &mut duration_state,
        &mut duration_model,
        &mut executor,
    );

    assert_eq!(turn_state.stop_reason_code(), Some("max_turns"));
    assert_eq!(tool_state.stop_reason_code(), Some("max_tool_calls"));
    assert_eq!(duration_state.stop_reason_code(), Some("max_duration"));
    assert!(
        [&turn_state, &tool_state, &duration_state]
            .into_iter()
            .all(|state| state.status() == IterativeLoopStatus::Exhausted
                && state.final_response().is_none())
    );
}

#[test]
fn iterative_loop_sources_stay_below_line_guards() {
    for (path, source) in [
        (
            "crates/desktoplab-agent-engine/src/iterative_loop.rs",
            include_str!("../src/iterative_loop.rs"),
        ),
        (
            "crates/desktoplab-agent-engine/src/iterative_protocol.rs",
            include_str!("../src/iterative_protocol.rs"),
        ),
        (
            "crates/desktoplab-agent-engine/src/iterative_state.rs",
            include_str!("../src/iterative_state.rs"),
        ),
    ] {
        check_logical_line_limit(path, source, 250)
            .expect("iterative agent loop sources should stay below the line-count guard");
    }
}
