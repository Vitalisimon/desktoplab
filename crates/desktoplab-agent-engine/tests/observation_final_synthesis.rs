use std::collections::VecDeque;

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopState, IterativeLoopStatus, IterativeModelAdapter,
    IterativeModelDecision, IterativeToolCall, IterativeToolExecutor, ToolObservation,
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

struct ObservationAwareModel;

impl IterativeModelAdapter for ObservationAwareModel {
    fn decide(&mut self, state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        if let Some(observation) = state.observations().last() {
            let path = observation.provenance().target().unwrap_or("unknown");
            let text = observation.output()["text"].as_str().unwrap_or_default();
            return Ok(IterativeModelDecision::final_response(format!(
                "{path} says: {text}"
            )));
        }
        Ok(IterativeModelDecision::tool_call(IterativeToolCall::new(
            "read-1",
            "desktoplab.read_file",
            json!({"path":"README.md"}),
        )))
    }
}

struct SearchAwareModel;

impl IterativeModelAdapter for SearchAwareModel {
    fn decide(&mut self, state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        if let Some(observation) = state.observations().last() {
            let path = observation.output()["matches"][0]["path"]
                .as_str()
                .unwrap_or("unknown");
            return Ok(IterativeModelDecision::final_response(format!(
                "The implementation is in {path}."
            )));
        }
        Ok(IterativeModelDecision::tool_call(IterativeToolCall::new(
            "search-1",
            "desktoplab.search_text",
            json!({"query":"IterativeAgentLoop","path":"crates"}),
        )))
    }
}

struct FixtureExecutor {
    output: serde_json::Value,
}

impl IterativeToolExecutor for FixtureExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::success(call, self.output.clone()))
    }
}

#[test]
fn read_observation_is_fed_back_for_natural_grounded_synthesis() {
    let mut state = IterativeLoopState::new("read-summary");
    let mut model = ObservationAwareModel;
    let mut executor = FixtureExecutor {
        output: json!({"text":"DesktopLab is a local agent runtime.","truncated":false}),
    };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert_eq!(
        state.final_response(),
        Some("README.md says: DesktopLab is a local agent runtime.")
    );
    let provenance = state.observations()[0].provenance();
    assert_eq!(provenance.source(), "desktoplab.read_file");
    assert_eq!(provenance.target(), Some("README.md"));
    assert!(!provenance.truncated());
}

#[test]
fn search_observation_supports_a_final_answer_with_real_file_location() {
    let mut state = IterativeLoopState::new("search-summary");
    let mut model = SearchAwareModel;
    let mut executor = FixtureExecutor {
        output: json!({
            "matches":[{"path":"crates/desktoplab-agent-engine/src/iterative_loop.rs"}],
            "truncated":false
        }),
    };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert_eq!(
        state.final_response(),
        Some("The implementation is in crates/desktoplab-agent-engine/src/iterative_loop.rs.")
    );
}

#[test]
fn large_observation_is_bounded_without_losing_exact_mutation_target() {
    let call = IterativeToolCall::new(
        "patch-1",
        "desktoplab.patch_file",
        json!({"path":"src/lib.rs","expected":"old","replacement":"new"}),
    );
    let observation = ToolObservation::success(
        &call,
        json!({"path":"src/lib.rs","diff":"x".repeat(100_000),"changed":true}),
    );

    assert_eq!(observation.provenance().target(), Some("src/lib.rs"));
    assert!(observation.provenance().truncated());
    assert!(observation.output().to_string().len() < 70_000);
    assert_eq!(observation.output()["changed"], true);
    assert_eq!(observation.output()["path"], "src/lib.rs");
}

#[test]
fn verbose_passing_test_keeps_machine_verifiable_completion_fields() {
    let call = IterativeToolCall::new(
        "test-large",
        "desktoplab.run_tests",
        json!({"command":"cargo test"}),
    );
    let observation = ToolObservation::success(
        &call,
        json!({
            "status":"exited",
            "passed":true,
            "exitCode":0,
            "stdout":"x".repeat(100_000),
            "stderr":""
        }),
    );

    assert!(observation.provenance().truncated());
    assert_eq!(observation.output()["status"], "exited");
    assert_eq!(observation.output()["passed"], true);
    assert_eq!(observation.output()["exitCode"], 0);
}

#[test]
fn raw_tool_envelopes_cannot_become_final_prose() {
    let mut state = IterativeLoopState::new("raw-final");
    let mut model = ScriptedModel {
        decisions: [IterativeModelDecision::final_response(
            r#"{"tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        )]
        .into(),
    };
    let mut executor = FixtureExecutor { output: json!({}) };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Failed);
    assert_eq!(state.stop_reason_code(), Some("invalid_final_response"));
    assert!(state.final_response().is_none());
}

#[test]
fn final_cannot_claim_tests_passed_without_passing_executor_evidence() {
    let test_call = IterativeToolCall::new(
        "test-1",
        "desktoplab.run_tests",
        json!({"command":"cargo test"}),
    );
    let mut state = IterativeLoopState::new("false-test-claim");
    let mut model = ScriptedModel {
        decisions: [
            IterativeModelDecision::tool_call(test_call),
            IterativeModelDecision::final_response("All tests passed."),
        ]
        .into(),
    };
    let mut executor = FixtureExecutor {
        output: json!({"passed":false,"exitCode":1,"stdout":"","stderr":"failure"}),
    };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Failed);
    assert_eq!(state.stop_reason_code(), Some("unsupported_test_claim"));
}

#[test]
fn passing_test_evidence_allows_supported_final_claim() {
    let test_call = IterativeToolCall::new(
        "test-1",
        "desktoplab.run_tests",
        json!({"command":"cargo test"}),
    );
    let mut state = IterativeLoopState::new("supported-test-claim");
    let mut model = ScriptedModel {
        decisions: [
            IterativeModelDecision::tool_call(test_call),
            IterativeModelDecision::final_response("All tests passed."),
        ]
        .into(),
    };
    let mut executor = FixtureExecutor {
        output: json!({"passed":true,"exitCode":0,"stdout":"ok","stderr":""}),
    };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
}

#[test]
fn executor_attested_terminal_test_allows_supported_final_claim() {
    let test_call = IterativeToolCall::new(
        "test-1",
        "desktoplab.run_terminal",
        json!({"command":"npm test"}),
    );
    let mut state = IterativeLoopState::new("supported-terminal-test-claim");
    let mut model = ScriptedModel {
        decisions: [
            IterativeModelDecision::tool_call(test_call),
            IterativeModelDecision::final_response("All tests passed."),
        ]
        .into(),
    };
    let mut executor = FixtureExecutor {
        output: json!({"passed":true,"exitCode":0,"stdout":"ok","stderr":""}),
    };

    IterativeAgentLoop::default().run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
}

#[test]
fn observation_and_synthesis_sources_stay_below_line_guards() {
    for (path, source) in [
        (
            "crates/desktoplab-agent-engine/src/observation.rs",
            include_str!("../src/observation.rs"),
        ),
        (
            "crates/desktoplab-agent-engine/src/final_response.rs",
            include_str!("../src/final_response.rs"),
        ),
    ] {
        check_logical_line_limit(path, source, 250)
            .expect("observation and synthesis source grew too large");
    }
}
