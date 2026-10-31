use super::{BackendDecisionAdapter, backend_messages};
use desktoplab_agent_engine::{
    DesktopLabToolRegistry, IterativeAgentLoop, IterativeLoopState, IterativeToolCall,
    IterativeToolExecutor, ToolObservation,
};
use desktoplab_backends::BackendMessage;
use xtask::check_logical_line_limit;

fn guidance(reason: &str, registry: &DesktopLabToolRegistry) -> String {
    let mut state = IterativeLoopState::new("session.schema-feedback");
    assert!(state.request_model_protocol_retry(reason));
    let messages = backend_messages("Inspect the workspace", &state, registry);
    match messages.last().unwrap() {
        BackendMessage::User(message) => message.clone(),
        _ => panic!("expected protocol feedback"),
    }
}

#[test]
fn missing_fields_are_explained_from_available_tool_schemas() {
    let all = DesktopLabToolRegistry::default();
    let registry = all
        .clone()
        .retaining(|tool| tool.id() == "desktoplab.clarify");
    let message = guidance("missing_argument:question", &registry);
    assert!(message.contains("desktoplab.clarify"));
    assert!(message.contains("question"));
    assert!(message.contains("blockedOn"));
    assert!(message.contains(r#""type":"string""#));
    assert!(message.contains("possible matches"));
    assert!(!message.contains("desktoplab.write_file"));

    let registry = all.retaining(|tool| tool.id() == "desktoplab.run_tests");
    let message = guidance("missing_argument:command", &registry);
    assert!(message.contains("desktoplab.run_tests"));
    assert!(message.contains("command"));
    assert!(!message.contains("desktoplab.clarify"));
}

#[test]
fn ambiguous_fields_remain_candidates_and_suppressed_tools_stay_hidden() {
    let registry = DesktopLabToolRegistry::default()
        .retaining(|tool| matches!(tool.id(), "desktoplab.read_file" | "desktoplab.write_file"));
    let message = super::recovery_guidance::missing_field_schema_guidance(
        "missing_argument:path",
        &registry,
        Some("desktoplab.write_file"),
    );
    assert!(message.contains("desktoplab.read_file"));
    assert!(!message.contains("desktoplab.write_file"));
    assert!(message.contains("never invent missing values"));
}

#[test]
fn no_schema_is_invented_for_an_unknown_field() {
    let registry =
        DesktopLabToolRegistry::default().retaining(|tool| tool.id() == "desktoplab.clarify");
    let message = guidance("missing_argument:nonexistent", &registry);
    assert!(!message.contains("Schema candidates:"));
}

#[test]
fn unavailable_tools_are_not_suggested_for_non_file_recovery() {
    let mut state = IterativeLoopState::new("session.no-navigation");
    let mut adapter = BackendDecisionAdapter::new("Inspect", |_| {
        Ok(
            r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"docs"}}"#
                .to_string(),
        )
    });
    IterativeAgentLoop::default().advance(&mut state, &mut adapter, &mut NonFileExecutor);
    let registry =
        DesktopLabToolRegistry::default().retaining(|tool| tool.id() == "desktoplab.read_file");
    let messages = backend_messages("Inspect", &state, &registry);
    let BackendMessage::User(message) = messages.last().unwrap() else {
        panic!("expected evidence guidance")
    };
    assert!(!message.contains("desktoplab.list_files"));
}

struct NonFileExecutor;

impl IterativeToolExecutor for NonFileExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Ok(ToolObservation::failure(call, "not_regular_file"))
    }
}

#[test]
fn non_file_failure_offers_general_navigation_without_inventing_a_path() {
    let mut state = IterativeLoopState::new("session.non-file");
    let mut adapter = BackendDecisionAdapter::new("Inspect", |_| {
        Ok(
            r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"docs"}}"#
                .to_string(),
        )
    });
    IterativeAgentLoop::default().advance(&mut state, &mut adapter, &mut NonFileExecutor);
    let messages = backend_messages("Inspect", &state, &DesktopLabToolRegistry::default());
    let BackendMessage::User(message) = messages.last().unwrap() else {
        panic!("expected evidence guidance")
    };
    assert!(message.contains("not a regular file"));
    assert!(message.contains("desktoplab.list_files"));
    assert!(!message.contains("README.md"));
}

#[test]
fn non_file_guidance_does_not_displace_failed_test_guidance() {
    let mut state = IterativeLoopState::new("session.mixed-failures");
    let mut test = BackendDecisionAdapter::new("Inspect", |_| {
        Ok(
            r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"cargo test"}}"#
                .to_string(),
        )
    });
    IterativeAgentLoop::default().advance(&mut state, &mut test, &mut NonFileExecutor);
    let mut read = BackendDecisionAdapter::new("Inspect", |_| {
        Ok(
            r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"docs"}}"#
                .to_string(),
        )
    });
    IterativeAgentLoop::default().advance(&mut state, &mut read, &mut NonFileExecutor);
    let messages = backend_messages("Inspect", &state, &DesktopLabToolRegistry::default());
    let BackendMessage::User(message) = messages.last().unwrap() else {
        panic!("expected evidence guidance")
    };
    assert!(message.contains("A failing test is diagnostic evidence"));
    assert!(message.contains("not a regular file"));
}

#[test]
fn recovery_guidance_tests_stay_focused() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_model_adapter/recovery_guidance_tests.rs",
        include_str!("recovery_guidance_tests.rs"),
        150,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/agent_model_adapter/recovery_guidance.rs",
        include_str!("recovery_guidance.rs"),
        100,
    )
    .unwrap();
}
