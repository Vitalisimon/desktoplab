use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint,
};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use xtask::check_logical_line_limit;

#[test]
fn lm_studio_execution_is_selected_through_capabilities() {
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available("http://127.0.0.1:1234"),
        BackendModelInventory::available(&["local-model"]),
    );
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["llm.chat", "api.openai-compatible.local"]),
        vec![backend.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Selected);
    assert_eq!(route.backend_id(), Some("backend.lm-studio"));
}

#[test]
fn endpoint_unavailability_is_normalized() {
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::unavailable("http://127.0.0.1:1234", "connection refused"),
        BackendModelInventory::available(&["local-model"]),
    );

    let result = backend.execute(BackendPrompt::new("local-model", "summarize"));

    assert!(!result.is_ready());
    assert_eq!(result.reason(), Some("endpoint_unavailable"));
}

#[test]
fn local_lm_studio_runtime_requires_no_provider_credential() {
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available("http://127.0.0.1:1234"),
        BackendModelInventory::available(&["local-model"]),
    );

    assert!(!backend.requires_provider_credential());
}

#[test]
fn lm_studio_execute_chat_posts_tools_and_returns_desktoplab_action_json() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test endpoint");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept lm studio request");
        let mut request = [0_u8; 8192];
        let count = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.contains("POST /v1/chat/completions"));
        assert!(request.contains("desktoplab.search_text"));
        let body = json!({
            "choices":[{
                "message":{
                    "content":"Cerco nel workspace.",
                    "tool_calls":[{
                        "function":{
                            "name":"desktoplab.search_text",
                            "arguments":"{\"query\":\"composer\"}"
                        }
                    }]
                }
            }]
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

    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["local-model"]),
    );
    let prompt =
        BackendPrompt::new("local-model", "cerca composer").with_tools(vec![search_text_schema()]);
    let output = backend
        .execute_chat(&prompt)
        .expect("lm studio chat should parse tool call");

    server.join().expect("test server should finish");
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["tool"], "desktoplab.search_text");
    assert_eq!(value["arguments"]["query"], "composer");
}

#[test]
fn lm_studio_backend_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/lm_studio_execution.rs",
            include_str!("../src/lm_studio_execution.rs"),
            280,
        ),
        (
            "crates/desktoplab-backends/tests/lm_studio_backed_execution.rs",
            include_str!("lm_studio_backed_execution.rs"),
            130,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("lm studio backend source should stay below the initial line-count guard");
    }
}

fn search_text_schema() -> BackendToolSchema {
    BackendToolSchema::new(
        "desktoplab.search_text",
        "Search text inside the workspace.",
        json!({
            "type":"object",
            "properties":{"query":{"type":"string"}},
            "required":["query"]
        }),
    )
}
