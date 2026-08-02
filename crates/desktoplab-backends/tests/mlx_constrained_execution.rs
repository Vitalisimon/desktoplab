use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint,
};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn mlx_payload_embeds_the_canonical_json_contract_without_native_tools() {
    let backend = backend("http://127.0.0.1:18080");
    let payload = backend.constrained_chat_completion_payload(&prompt());

    assert_eq!(payload["model"], "mlx-model");
    assert!(payload.get("tools").is_none());
    assert!(payload.get("tool_choice").is_none());
    let contract = payload["messages"][0]["content"].as_str().unwrap();
    assert!(contract.contains("Return exactly one JSON object"));
    assert!(contract.contains("\"name\":\"desktoplab.search_text\""));
    assert!(contract.contains("\"arguments\""));
}

#[test]
fn mlx_agent_payload_is_deterministic_and_reserves_completion_budget() {
    let payload = backend("http://127.0.0.1:18080").constrained_chat_completion_payload(&prompt());

    assert_eq!(payload["temperature"], 0);
    assert_eq!(payload["max_tokens"], 2048);
}

#[test]
fn mlx_constrained_response_becomes_a_canonical_desktoplab_action() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 16384];
        let count = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.contains("Return exactly one JSON object"));
        assert!(!request.contains("\"tool_choice\""));
        let body = json!({
            "choices":[{"message":{"content":
                "{\"name\":\"desktoplab.search_text\",\"arguments\":{\"query\":\"composer\"}}"
            }}]
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    let output = backend(&endpoint)
        .execute_constrained_chat(&prompt())
        .expect("MLX constrained output should be normalized");
    server.join().unwrap();

    let action: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(action["tool"], "desktoplab.search_text");
    assert_eq!(action["arguments"]["query"], "composer");
}

#[test]
fn mlx_constrained_response_rejects_prose_and_markdown() {
    for content in [
        "Use desktoplab.search_text",
        "```json\n{\"name\":\"desktoplab.search_text\",\"arguments\":{}}\n```",
    ] {
        assert_eq!(
            desktoplab_backends::parse_constrained_tool_text(content),
            Err("provider_constrained_tool_invalid_json".to_string())
        );
    }
}

#[test]
fn mlx_constrained_execution_test_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/mlx_constrained_execution.rs",
        include_str!("mlx_constrained_execution.rs"),
        130,
    )
    .expect("MLX constrained execution tests should stay focused");
}

fn backend(endpoint: &str) -> LmStudioExecutionBackend {
    LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["mlx-model"]),
    )
}

fn prompt() -> BackendPrompt {
    BackendPrompt::new("mlx-model", "find composer").with_tools(vec![BackendToolSchema::new(
        "desktoplab.search_text",
        "Search workspace text.",
        json!({
            "type":"object",
            "properties":{"query":{"type":"string"}},
            "required":["query"]
        }),
    )])
}
