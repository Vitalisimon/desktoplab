use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use desktoplab_backends::{
    BackendModelCapabilities, BackendModelInventory, BackendPrompt, BackendToolSchema,
    ModelCapabilityState, OllamaExecutionBackend, OllamaModelCapabilityResolver,
};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use serde_json::json;

#[test]
fn ollama_capabilities_are_cached_by_digest_and_invalidated_on_change() {
    let tags_v1 = tags("qwen:7b", "digest-v1");
    let tags_v2 = tags("qwen:7b", "digest-v2");
    let show_tools = json!({
        "capabilities":["completion","tools","thinking"],
        "model_info":{"qwen.context_length":8192},
        "parameters":"temperature 0.2\nnum_ctx 32768"
    });
    let show_chat_only = json!({
        "capabilities":["completion"],
        "model_info":{"qwen.context_length":65536}
    });
    let (endpoint, server) = scripted_server(vec![
        ("GET /api/tags", tags_v1.clone()),
        ("POST /api/show", show_tools),
        ("GET /api/tags", tags_v1),
        ("GET /api/tags", tags_v2),
        ("POST /api/show", show_chat_only),
    ]);
    let resolver = OllamaModelCapabilityResolver::default();

    let first = resolver.resolve(&endpoint, "qwen:7b").unwrap();
    let cached = resolver.resolve(&endpoint, "qwen:7b").unwrap();
    let changed = resolver.resolve(&endpoint, "qwen:7b").unwrap();

    server.join().unwrap();
    assert_eq!(
        first.capability_state("tools"),
        ModelCapabilityState::Confirmed
    );
    assert_eq!(first.context_window(), Some(32_768));
    assert_eq!(cached.fingerprint(), first.fingerprint());
    assert_ne!(changed.fingerprint(), first.fingerprint());
    assert_eq!(
        changed.capability_state("tools"),
        ModelCapabilityState::Unsupported
    );
}

#[test]
fn missing_ollama_capability_metadata_never_implies_tool_support() {
    let (endpoint, server) = scripted_server(vec![
        ("GET /api/tags", tags("legacy:7b", "digest-legacy")),
        (
            "POST /api/show",
            json!({"model_info":{"legacy.context_length":4096}}),
        ),
    ]);

    let capabilities = OllamaModelCapabilityResolver::default()
        .resolve(&endpoint, "legacy:7b")
        .unwrap();

    server.join().unwrap();
    assert_eq!(
        capabilities.capability_state("tools"),
        ModelCapabilityState::ProbeRequired
    );
}

#[test]
fn tool_routing_fails_closed_without_model_level_capability_evidence() {
    let prompt =
        BackendPrompt::new("chat-only:7b", "read README").with_tools(vec![read_file_schema()]);
    let unverified =
        OllamaExecutionBackend::new(BackendModelInventory::available(&["chat-only:7b"]));
    let unsupported =
        OllamaExecutionBackend::new(BackendModelInventory::available(&["chat-only:7b"]))
            .with_model_capabilities([BackendModelCapabilities::reported(
                "backend.ollama",
                "chat-only:7b",
                Some("digest-chat".to_string()),
                Some(8_192),
                ["completion"],
            )]);

    assert_eq!(
        unverified.execute(prompt.clone()).reason(),
        Some("model_tool_capability_unverified")
    );
    assert_eq!(
        unsupported.execute(prompt).reason(),
        Some("model_native_tools_unsupported")
    );
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["agent.protocol.native_tool_calls"]),
        vec![unsupported.route_candidate_for_model("chat-only:7b")],
    );
    assert_eq!(route.status(), RouteStatus::Blocked);
}

#[test]
fn capability_discovery_sources_stay_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-backends/src/model_capabilities.rs",
            include_str!("../src/model_capabilities.rs"),
            230,
        ),
        (
            "crates/desktoplab-backends/src/ollama_capabilities.rs",
            include_str!("../src/ollama_capabilities.rs"),
            190,
        ),
        (
            "crates/desktoplab-backends/tests/ollama_model_capabilities.rs",
            include_str!("ollama_model_capabilities.rs"),
            220,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit).unwrap();
    }
}

fn tags(model: &str, digest: &str) -> serde_json::Value {
    json!({"models":[{"name":model,"digest":digest}]})
}

fn read_file_schema() -> BackendToolSchema {
    BackendToolSchema::new(
        "desktoplab.read_file",
        "Read a workspace file.",
        json!({"type":"object","properties":{"path":{"type":"string"}}}),
    )
}

fn scripted_server(
    responses: Vec<(&'static str, serde_json::Value)>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        for (expected_request, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let request_line = read_request(&stream);
            assert!(
                request_line.starts_with(expected_request),
                "expected {expected_request}, got {request_line}"
            );
            write_json(&mut stream, &body);
        }
    });
    (endpoint, server)
}

fn read_request(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        if header == "\r\n" || header.is_empty() {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    request_line
}

fn write_json(stream: &mut TcpStream, body: &serde_json::Value) {
    let body = body.to_string();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .unwrap();
}
