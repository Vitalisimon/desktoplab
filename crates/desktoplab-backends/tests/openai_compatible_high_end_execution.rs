use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LocalEndpoint,
    OpenAiCompatibleLocalExecutionBackend,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn high_end_openai_compatible_backend_executes_native_tool_calls() {
    let endpoint = serve_once(
        r#"{"choices":[{"message":{"content":"","tool_calls":[{"id":"call.1","type":"function","function":{"name":"desktoplab.read_file","arguments":"{\"path\":\"README.md\"}"}}]}}]}"#,
    );
    let backend = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.high-end-local",
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["model.frontier"]),
    );
    let prompt = BackendPrompt::new("model.frontier", "Read the repository").with_tools(vec![
        BackendToolSchema::new(
            "desktoplab.read_file",
            "Read one workspace file.",
            json!({"type":"object","properties":{"path":{"type":"string"}}}),
        ),
    ]);

    let output = backend
        .execute_chat(&prompt)
        .expect("live request should pass");
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["tool"], "desktoplab.read_file");
    assert_eq!(value["arguments"]["path"], "README.md");
}

fn serve_once(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("desktoplab.read_file"));
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    format!("http://{address}")
}
