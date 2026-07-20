use desktoplab_backends::{
    BackendMessage, BackendPrompt, LmStudioExecutionBackend, LocalEndpoint, OllamaExecutionBackend,
};
use serde_json::json;

#[test]
fn backend_payloads_preserve_native_tool_call_history_and_ids() {
    let messages = vec![
        BackendMessage::user("Inspect the repository."),
        BackendMessage::assistant_tool_call(
            "call.read.1",
            "desktoplab.read_file",
            json!({"path":"README.md"}),
        ),
        BackendMessage::tool_result(
            "call.read.1",
            "desktoplab.read_file",
            json!({"content":"# DesktopLab"}),
        ),
    ];
    let prompt = BackendPrompt::new("model.local", "unused").with_messages(messages);
    let ollama = OllamaExecutionBackend::chat_payload(&prompt);
    let lm_studio = LmStudioExecutionBackend::new(
        LocalEndpoint::available("http://127.0.0.1:1234"),
        desktoplab_backends::BackendModelInventory::available(&["model.local"]),
    )
    .chat_completion_payload(&prompt);

    for payload in [&ollama, &lm_studio] {
        assert_eq!(payload["messages"][1]["tool_calls"][0]["id"], "call.read.1");
        assert_eq!(payload["messages"][2]["tool_call_id"], "call.read.1");
        assert!(payload["messages"][2].get("tool_name").is_none());
        assert!(
            payload["messages"][2]["content"]
                .as_str()
                .unwrap()
                .contains("DesktopLab")
        );
    }
    assert!(ollama["messages"][1]["tool_calls"][0]["function"]["arguments"].is_object());
    assert_eq!(
        lm_studio["messages"][1]["tool_calls"][0]["function"]["arguments"],
        r#"{"path":"README.md"}"#
    );
    assert!(lm_studio["messages"][1]["content"].is_null());
}
