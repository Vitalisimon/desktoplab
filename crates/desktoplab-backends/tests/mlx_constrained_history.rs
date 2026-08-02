use desktoplab_backends::{
    BackendMessage, BackendModelInventory, BackendPrompt, BackendToolSchema,
    LmStudioExecutionBackend, LocalEndpoint,
};
use serde_json::{Value, json};
use xtask::check_logical_line_limit;

#[test]
fn constrained_history_serializes_calls_and_results_as_text() {
    let payload = backend().constrained_chat_completion_payload(&prompt());
    let messages = payload["messages"].as_array().unwrap();
    let historical_call = &messages[2];
    let historical_result = &messages[3];

    assert_eq!(historical_call["role"], "assistant");
    assert!(historical_call.get("tool_calls").is_none());
    let call: Value = serde_json::from_str(historical_call["content"].as_str().unwrap()).unwrap();
    assert_eq!(call["name"], "desktoplab.list_files");
    assert_eq!(call["arguments"]["path"], ".");

    assert_eq!(historical_result["role"], "user");
    assert!(historical_result.get("tool_call_id").is_none());
    let result = historical_result["content"].as_str().unwrap();
    assert!(result.contains("call.1"));
    assert!(result.contains("desktoplab.list_files"));
    assert!(result.contains("README.md"));
}

#[test]
fn native_history_retains_openai_tool_messages() {
    let payload = backend().chat_completion_payload(&prompt());
    let messages = payload["messages"].as_array().unwrap();

    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(
        messages[1]["tool_calls"][0]["function"]["name"],
        "desktoplab.list_files"
    );
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[2]["tool_call_id"], "call.1");
}

#[test]
fn constrained_history_test_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/mlx_constrained_history.rs",
        include_str!("mlx_constrained_history.rs"),
        100,
    )
    .expect("MLX constrained history tests should stay focused");
}

fn backend() -> LmStudioExecutionBackend {
    LmStudioExecutionBackend::new(
        LocalEndpoint::available("http://127.0.0.1:18080"),
        BackendModelInventory::available(&["mlx-model"]),
    )
}

fn prompt() -> BackendPrompt {
    BackendPrompt::new("mlx-model", "Inspect this repository")
        .with_messages(vec![
            BackendMessage::user("Inspect this repository"),
            BackendMessage::assistant_tool_call(
                "call.1",
                "desktoplab.list_files",
                json!({"path":"."}),
            ),
            BackendMessage::tool_result(
                "call.1",
                "desktoplab.list_files",
                json!({"entries":["README.md"]}),
            ),
        ])
        .with_tools(vec![BackendToolSchema::new(
            "desktoplab.list_files",
            "List workspace files.",
            json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        )])
}
