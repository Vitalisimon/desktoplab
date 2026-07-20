use std::fs;

use desktoplab_agent_engine::IterativeToolExecutor;
use desktoplab_control_plane::{
    BackendToolProtocolClass, BackendToolProtocolHealth, CanonicalAgentToolExecutor,
    CanonicalExecutionApproval, ToolProtocolError, backend_tool_protocol_class,
    normalize_backend_tool_output,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn native_and_constrained_json_use_the_same_canonical_executor() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("README.md"), "same executor\n").unwrap();
    let native = normalize_backend_tool_output(
        r#"{"id":"native-1","function":{"name":"desktoplab.read_file","arguments":"{\"path\":\"README.md\"}"}}"#,
        BackendToolProtocolClass::NativeTool,
        "unused",
    )
    .unwrap();
    let fallback = normalize_backend_tool_output(
        r#"{"tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        BackendToolProtocolClass::ConstrainedJson,
        "fallback-1",
    )
    .unwrap();
    let mut executor = CanonicalAgentToolExecutor::new(
        repo.path(),
        "workspace",
        "session",
        CanonicalExecutionApproval::Pending,
    );

    let native_output = executor.execute(&native).unwrap();
    let fallback_output = executor.execute(&fallback).unwrap();

    assert_eq!(native.name(), fallback.name());
    assert_eq!(native.arguments(), fallback.arguments());
    assert_eq!(native_output.output(), fallback_output.output());
}

#[test]
fn malformed_actions_downgrade_route_to_chat_only() {
    let mut health = BackendToolProtocolHealth::new(BackendToolProtocolClass::ConstrainedJson, 2);
    for _ in 0..2 {
        let result = normalize_backend_tool_output("not-json", health.effective(), "invalid");
        assert_eq!(result, Err(ToolProtocolError::MalformedOutput));
        health.record_invalid_action();
    }

    assert_eq!(health.invalid_actions(), 2);
    assert_eq!(health.effective(), BackendToolProtocolClass::ChatOnly);
    assert!(!health.effective().supports_full_coding_agent());
}

#[test]
fn chat_only_backend_rejects_tool_output_and_is_not_agent_eligible() {
    let class = backend_tool_protocol_class("backend.codex");
    let result = normalize_backend_tool_output(
        r#"{"tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        class,
        "blocked",
    );

    assert_eq!(class, BackendToolProtocolClass::ChatOnly);
    assert_eq!(result, Err(ToolProtocolError::ChatOnly));
    assert!(!class.supports_full_coding_agent());
}

#[test]
fn protocol_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/execution_tool_calling.rs",
        include_str!("../src/execution_tool_calling.rs"),
        250,
    )
    .expect("backend tool protocol source grew too large");
}
