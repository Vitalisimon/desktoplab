use std::collections::VecDeque;

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopState, IterativeModelAdapter, IterativeModelDecision,
    IterativeToolCall, IterativeToolExecutor, ToolObservation,
};
use serde_json::json;
use xtask::check_logical_line_limit;

struct ScriptedModel {
    decisions: VecDeque<IterativeModelDecision>,
}

impl IterativeModelAdapter for ScriptedModel {
    fn decide(&mut self, _state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        self.decisions
            .pop_front()
            .ok_or_else(|| "script_exhausted".to_string())
    }
}

struct Executor;

impl IterativeToolExecutor for Executor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::success(
            call,
            json!({
                "text": format!("token {}{}", "ghp_", "123456789012345678901234567890"),
                "truncated":true,
                "exitCode":0
            }),
        ))
    }
}

#[test]
fn trace_is_ordered_correlated_redacted_and_restart_safe() {
    let call = IterativeToolCall::new(
        "call-1",
        "desktoplab.read_file",
        json!({"path":"/Users/private/repo/README.md"}),
    );
    let mut model = ScriptedModel {
        decisions: [
            IterativeModelDecision::tool_call(call),
            IterativeModelDecision::final_response("Read completed with executor evidence."),
        ]
        .into(),
    };
    let mut state = IterativeLoopState::new("trace-session");
    IterativeAgentLoop::default().run(&mut state, &mut model, &mut Executor);

    let events = state.trace().events();
    assert_eq!(
        events.iter().map(|event| event.kind()).collect::<Vec<_>>(),
        [
            "model_turn",
            "tool_requested",
            "tool_observed",
            "model_turn",
            "completed"
        ]
    );
    assert_eq!(events[1].parent_event_id(), Some(events[0].event_id()));
    assert_eq!(events[2].parent_event_id(), Some(events[1].event_id()));
    assert_eq!(events[2].success(), Some(true));
    assert!(events[2].truncated());
    assert!(events[2].duration_ms().is_some());

    let jsonl = state.trace().to_jsonl().expect("trace should serialize");
    assert!(!jsonl.contains("ghp_"));
    assert!(!jsonl.contains("/Users/private"));
    let restored = IterativeLoopState::from_json(&state.to_json().unwrap()).unwrap();
    assert_eq!(restored.trace().events().len(), events.len());
    assert_eq!(
        restored.trace().events()[2].event_id(),
        events[2].event_id()
    );
}

#[test]
fn trace_uses_canonical_effects_for_current_tool_names() {
    for (ordinal, name, expected_mutation) in [
        (1, "desktoplab.patch_file", true),
        (2, "desktoplab.create_directory", true),
        (3, "desktoplab.write_process_stdin", true),
        (4, "desktoplab.create_checkpoint", true),
        (5, "desktoplab.update_plan", true),
        (6, "desktoplab.read_file", false),
        (7, "desktoplab.get_subagent", false),
    ] {
        let call = IterativeToolCall::new(format!("call-{ordinal}"), name, json!({}));
        let mut model = ScriptedModel {
            decisions: [
                IterativeModelDecision::tool_call(call),
                IterativeModelDecision::final_response("Executor evidence recorded."),
            ]
            .into(),
        };
        let mut state = IterativeLoopState::new(format!("trace-{ordinal}"));
        IterativeAgentLoop::default().run(&mut state, &mut model, &mut Executor);
        assert_eq!(
            state.trace().events()[1].mutation(),
            expected_mutation,
            "{name}"
        );
        assert_eq!(
            state.trace().events()[2].mutation(),
            expected_mutation,
            "{name}"
        );
    }
}

#[test]
fn trace_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/trace.rs",
        include_str!("../src/trace.rs"),
        320,
    )
    .expect("trace source should stay focused");
}
