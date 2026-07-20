use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;

use desktoplab_backends::{BackendModelInventory, BackendPrompt, OllamaExecutionBackend};
use xtask::check_logical_line_limit;

#[test]
fn ollama_stream_emits_provider_deltas_and_aggregates_content() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test endpoint");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept ollama request");
        let mut request = [0_u8; 4096];
        let count = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.contains("POST /api/chat"));
        assert!(request.contains("\"stream\":true"));
        let body = concat!(
            "{\"message\":{\"content\":\"Hello \"},\"done\":false}\n",
            "{\"message\":{\"content\":\"world\"},\"done\":true}\n"
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/x-ndjson\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write streaming response");
    });

    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["qwen:7b"]));
    let mut deltas = Vec::new();
    let output = backend
        .execute_chat_stream(
            &endpoint,
            &BackendPrompt::new("qwen:7b", "hello"),
            &AtomicBool::new(false),
            |delta| deltas.push(delta.to_string()),
        )
        .expect("stream should complete");

    server.join().expect("test server should finish");
    assert_eq!(deltas, ["Hello ", "world"]);
    assert_eq!(output, "Hello world");
}

#[test]
fn ollama_stream_cancellation_fails_before_network_egress() {
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["qwen:7b"]));
    let error = backend
        .execute_chat_stream(
            "http://127.0.0.1:1",
            &BackendPrompt::new("qwen:7b", "hello"),
            &AtomicBool::new(true),
            |_| {},
        )
        .expect_err("cancelled request must not start");

    assert_eq!(error, "agent_cancelled");
}

#[test]
fn ollama_streaming_sources_stay_below_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/ollama_stream.rs",
            include_str!("../src/ollama_stream.rs"),
            90,
        ),
        (
            "crates/desktoplab-backends/tests/ollama_streaming.rs",
            include_str!("ollama_streaming.rs"),
            110,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("Ollama streaming source should stay focused");
    }
}
