use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;

use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint, OpenAiCompatibleLocalExecutionBackend,
};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn lm_studio_stream_emits_exact_sse_content_deltas() {
    let endpoint = serve_sse(concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello \"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\n",
        "data: [DONE]\n\n"
    ));
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["local-model"]),
    );
    let mut deltas = Vec::new();
    let output = backend
        .execute_chat_stream(
            &BackendPrompt::new("local-model", "hello"),
            &AtomicBool::new(false),
            |delta| deltas.push(delta.to_string()),
        )
        .expect("LM Studio stream should complete");

    assert_eq!(deltas, ["Hello ", "world"]);
    assert_eq!(output, "Hello world");
}

#[test]
fn high_end_stream_reassembles_native_tool_call_fragments() {
    let endpoint = serve_sse(concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call.1\",\"type\":\"function\",\"function\":{\"name\":\"desktoplab.read_\",\"arguments\":\"{\\\"path\\\":\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"file\",\"arguments\":\"\\\"README.md\\\"}\"}}]}}]}\n\n",
        "data: [DONE]\n\n"
    ));
    let backend = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.high-end-local",
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["model.frontier"]),
    );
    let prompt = BackendPrompt::new("model.frontier", "Read README").with_tools(vec![
        BackendToolSchema::new(
            "desktoplab.read_file",
            "Read one workspace file.",
            json!({"type":"object","properties":{"path":{"type":"string"}}}),
        ),
    ]);
    let output = backend
        .execute_chat_stream(&prompt, &AtomicBool::new(false), |_| {})
        .expect("high-end stream should parse the tool call");
    let action: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(action["tool"], "desktoplab.read_file");
    assert_eq!(action["arguments"]["path"], "README.md");
}

#[test]
fn openai_compatible_stream_cancellation_prevents_egress() {
    let backend = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.high-end-local",
        LocalEndpoint::available("http://127.0.0.1:1"),
        BackendModelInventory::available(&["model.frontier"]),
    );
    let error = backend
        .execute_chat_stream(
            &BackendPrompt::new("model.frontier", "hello"),
            &AtomicBool::new(true),
            |_| {},
        )
        .expect_err("cancelled stream must not start");

    assert_eq!(error, "agent_cancelled");
}

#[test]
fn openai_streaming_sources_stay_below_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/openai_compatible_stream.rs",
            include_str!("../src/openai_compatible_stream.rs"),
            160,
        ),
        (
            "crates/desktoplab-backends/tests/openai_compatible_streaming.rs",
            include_str!("openai_compatible_streaming.rs"),
            150,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("OpenAI-compatible streaming source should stay focused");
    }
}

fn serve_sse(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 8192];
        let count = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("\"stream\":true"));
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    format!("http://{address}")
}
