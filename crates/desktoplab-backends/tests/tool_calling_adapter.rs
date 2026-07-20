use desktoplab_backends::{
    BackendPrompt, BackendToolCallEvidence, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint, OllamaExecutionBackend, parse_ollama_tool_response,
    parse_openai_compatible_tool_response,
};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn ollama_payload_includes_desktoplab_tools() {
    let prompt = BackendPrompt::new("qwen2.5-coder:7b", "trova il composer")
        .with_tools(vec![read_file_schema()]);
    let payload = OllamaExecutionBackend::chat_payload(&prompt);

    assert_eq!(payload["model"], "qwen2.5-coder:7b");
    assert_eq!(payload["messages"][0]["content"], "trova il composer");
    assert_eq!(
        payload["tools"][0]["function"]["name"],
        "desktoplab.read_file"
    );
    assert_eq!(
        payload["tools"][0]["function"]["description"],
        "Read a workspace file after DesktopLab policy checks."
    );
    assert_eq!(payload["stream"], false);
}

#[test]
fn openai_compatible_payload_uses_same_tool_schema() {
    let prompt =
        BackendPrompt::new("local-model", "leggi prova.md").with_tools(vec![read_file_schema()]);
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available("http://127.0.0.1:1234"),
        desktoplab_backends::BackendModelInventory::available(&["local-model"]),
    );
    let payload = backend.chat_completion_payload(&prompt);

    assert_eq!(payload["model"], "local-model");
    assert_eq!(
        payload["tools"][0]["function"]["name"],
        "desktoplab.read_file"
    );
    assert_eq!(payload["tool_choice"], "auto");
}

#[test]
fn native_tool_calls_are_parsed_without_losing_assistant_text() {
    let response = json!({
        "message":{
            "content":"Controllo il file.",
            "tool_calls":[{
                "id":"call_1",
                "function":{
                    "name":"desktoplab.read_file",
                    "arguments":{"path":"prova.md"}
                }
            }]
        }
    });
    let parsed = parse_ollama_tool_response(
        &response,
        BackendToolCallEvidence::native(
            "backend.ollama",
            "qwen2.5-coder:7b",
            "http://127.0.0.1:11434/api/chat",
            true,
        ),
    );

    assert_eq!(parsed.assistant_text(), Some("Controllo il file."));
    assert_eq!(parsed.tool_calls()[0].id(), Some("call_1"));
    assert_eq!(parsed.tool_calls()[0].name(), "desktoplab.read_file");
    assert_eq!(parsed.tool_calls()[0].arguments()["path"], "prova.md");
    assert!(parsed.evidence().native_tool_calls());
}

#[test]
fn openai_compatible_tool_calls_parse_json_argument_strings() {
    let response = json!({
        "choices":[{
            "message":{
                "content":"Cerco nel workspace.",
                "tool_calls":[{
                    "id":"call_search",
                    "type":"function",
                    "function":{
                        "name":"desktoplab.search_text",
                        "arguments":"{\"query\":\"composer\"}"
                    }
                }]
            }
        }]
    });
    let parsed = parse_openai_compatible_tool_response(
        &response,
        BackendToolCallEvidence::native(
            "backend.lm-studio",
            "local-model",
            "http://127.0.0.1:1234/v1/chat/completions",
            false,
        ),
    );

    assert_eq!(parsed.assistant_text(), Some("Cerco nel workspace."));
    assert_eq!(parsed.tool_calls()[0].name(), "desktoplab.search_text");
    assert_eq!(parsed.tool_calls()[0].arguments()["query"], "composer");
    assert!(!parsed.evidence().streaming_supported());
}

#[test]
fn tool_call_argument_strings_fail_closed_when_json_is_not_exact() {
    let concatenated =
        openai_response_with_arguments("{\"path\":\"README.md\"}{\"path\":\"duplicate.md\"}");
    let mixed = openai_response_with_arguments("Use this {\"path\":\"src/lib.rs\"} now");
    let malformed = openai_response_with_arguments("{\"path\":\"README.md\"");

    let concatenated = parse_openai_compatible_tool_response(
        &concatenated,
        BackendToolCallEvidence::fallback("backend.test", "model", "test"),
    );
    let mixed = parse_openai_compatible_tool_response(
        &mixed,
        BackendToolCallEvidence::fallback("backend.test", "model", "test"),
    );
    let malformed = parse_openai_compatible_tool_response(
        &malformed,
        BackendToolCallEvidence::fallback("backend.test", "model", "test"),
    );

    assert_eq!(
        concatenated.protocol_error(),
        Some("provider_tool_arguments_invalid_json")
    );
    assert_eq!(
        mixed.protocol_error(),
        Some("provider_tool_arguments_invalid_json")
    );
    assert_eq!(
        malformed.protocol_error(),
        Some("provider_tool_arguments_invalid_json")
    );
    assert!(concatenated.tool_calls().is_empty());
    assert!(mixed.tool_calls().is_empty());
    assert!(malformed.tool_calls().is_empty());
}

#[test]
fn fallback_evidence_is_explicit_when_backend_has_no_native_tools() {
    let evidence = BackendToolCallEvidence::fallback(
        "backend.mlx-lm",
        "mlx-community/model",
        "native_tool_calls_unavailable",
    );

    assert!(!evidence.native_tool_calls());
    assert_eq!(
        evidence.fallback_reason(),
        Some("native_tool_calls_unavailable")
    );
}

#[test]
fn tool_calling_adapter_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/tool_calling.rs",
            include_str!("../src/tool_calling.rs"),
            260,
        ),
        (
            "crates/desktoplab-backends/tests/tool_calling_adapter.rs",
            include_str!("tool_calling_adapter.rs"),
            210,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("tool calling adapter files should stay focused");
    }
}

fn read_file_schema() -> BackendToolSchema {
    BackendToolSchema::new(
        "desktoplab.read_file",
        "Read a workspace file after DesktopLab policy checks.",
        json!({
            "type":"object",
            "properties":{"path":{"type":"string"}},
            "required":["path"]
        }),
    )
}

fn openai_response_with_arguments(arguments: &str) -> serde_json::Value {
    json!({
        "choices":[{
            "message":{
                "content":"Leggo file.",
                "tool_calls":[{
                    "id":"call_read",
                    "type":"function",
                    "function":{
                        "name":"desktoplab.read_file",
                        "arguments":arguments
                    }
                }]
            }
        }]
    })
}
