use desktoplab_agent_session::SessionOwner;
use desktoplab_backends::{
    BackendModelCapabilities, BackendModelInventory, BackendPrompt, BackendToolSchema,
    OllamaExecutionBackend,
};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use xtask::check_logical_line_limit;

#[test]
fn ollama_execution_is_selected_through_capabilities() {
    let backend = tool_capable_backend("qwen3:8b");
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&[
            "llm.chat",
            "runtime.ollama",
            "agent.protocol.native_tool_calls",
        ]),
        vec![backend.route_candidate_for_model("qwen3:8b")],
    );

    assert_eq!(route.status(), RouteStatus::Selected);
    assert_eq!(route.backend_id(), Some("backend.ollama"));
}

#[test]
fn unavailable_ollama_model_blocks_execution_readiness() {
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["qwen3:8b"]));
    let result = backend.execute(BackendPrompt::new("devstral:24b", "explain repo"));

    assert!(!result.is_ready());
    assert_eq!(result.reason(), Some("model_unavailable"));
}

#[test]
fn ollama_backed_session_remains_desktoplab_owned() {
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["qwen3:8b"]));
    let session = backend.create_session("session.ollama");

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), "backend.ollama");
}

#[test]
fn ollama_execute_chat_posts_tools_and_returns_desktoplab_action_json() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test endpoint");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept ollama request");
        let mut request = [0_u8; 8192];
        let count = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.contains("POST /api/chat"));
        assert!(request.contains("desktoplab.read_file") && request.contains(r#""num_ctx":32768"#));
        let body = json!({
            "message":{
                "content":"Leggo il file.",
                "tool_calls":[{
                    "function":{
                        "name":"desktoplab.read_file",
                        "arguments":{"path":"README.md"}
                    }
                }]
            }
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let backend = tool_capable_backend("qwen:7b");
    let prompt = BackendPrompt::new("qwen:7b", "leggi README")
        .with_tools(vec![read_file_schema()])
        .with_context_window_tokens(32_768);
    let output = backend
        .execute_chat(&endpoint, &prompt)
        .expect("ollama chat should parse tool call");

    server.join().expect("test server should finish");
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["tool"], "desktoplab.read_file");
    assert_eq!(value["arguments"]["path"], "README.md");
    assert!(value.get("id").is_none());
}

#[test]
fn ollama_backend_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-backends/src/ollama_execution.rs",
            include_str!("../src/ollama_execution.rs"),
            280,
        ),
        (
            "crates/desktoplab-backends/tests/ollama_backed_execution.rs",
            include_str!("ollama_backed_execution.rs"),
            140,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("ollama backend source should stay below the initial line-count guard");
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

fn tool_capable_backend(model: &str) -> OllamaExecutionBackend {
    let profile = BackendModelCapabilities::reported(
        "backend.ollama",
        model,
        Some("digest-tools".to_string()),
        Some(32_768),
        ["completion", "tools"],
    );
    let certification =
        desktoplab_backends::ModelToolProtocolCertification::certified(profile.fingerprint());
    OllamaExecutionBackend::new(BackendModelInventory::available(&[model]))
        .with_model_capabilities([profile.with_tool_protocol_certification(certification)])
}
