use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;

use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint,
};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn retry_implementation_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-backends/src/mlx_constrained_retry.rs",
        include_str!("../src/mlx_constrained_retry.rs"),
        100,
    )
    .unwrap();
}

#[test]
fn empty_truncated_constrained_response_retries_once_without_thinking() {
    let (endpoint, worker) = server(false);
    let result = backend(&endpoint)
        .execute_constrained_chat(&prompt())
        .expect("retry should succeed");
    worker.join().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result).unwrap()["tool"],
        "desktoplab.read_file"
    );
}

#[test]
fn empty_truncated_stream_retries_without_emitting_reasoning() {
    let (endpoint, worker) = server(true);
    let mut deltas = Vec::new();
    let result = backend(&endpoint)
        .execute_constrained_chat_stream(&prompt(), &AtomicBool::new(false), |delta| {
            deltas.push(delta.to_string())
        })
        .expect("retry should succeed");
    worker.join().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result).unwrap()["tool"],
        "desktoplab.read_file"
    );
    assert_eq!(deltas.len(), 1);
    assert!(!deltas[0].contains("private reasoning"));
}

fn backend(endpoint: &str) -> LmStudioExecutionBackend {
    LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["model"]),
    )
}

fn prompt() -> BackendPrompt {
    BackendPrompt::new("model", "Read a file")
        .with_request_timeout_seconds(10)
        .with_tools(vec![BackendToolSchema::new(
            "desktoplab.read_file",
            "Read a file",
            json!({"type":"object"}),
        )])
}

fn server(streaming: bool) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        for attempt in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            if attempt == 1 {
                assert_eq!(request["chat_template_kwargs"]["enable_thinking"], false);
            }
            let body = if streaming {
                if attempt == 0 {
                    "data: {\"choices\":[{\"delta\":{\"reasoning\":\"private reasoning\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n"
                } else {
                    "data: {\"choices\":[{\"delta\":{\"content\":\"{\\\"name\\\":\\\"desktoplab.read_file\\\",\\\"arguments\\\":{}}\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
                }
            } else if attempt == 0 {
                "{\"choices\":[{\"message\":{\"reasoning\":\"private reasoning\"},\"finish_reason\":\"length\"}]}"
            } else {
                "{\"choices\":[{\"message\":{\"content\":\"{\\\"name\\\":\\\"desktoplab.read_file\\\",\\\"arguments\\\":{}}\"},\"finish_reason\":\"stop\"}]}"
            };
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", if streaming {"text/event-stream"} else {"application/json"}, body.len(), body).unwrap();
        }
    });
    (endpoint, worker)
}

fn read_request(stream: &mut std::net::TcpStream) -> serde_json::Value {
    let mut reader = BufReader::new(stream);
    let mut length = None;
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "incomplete request"
        );
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
    }
    let mut body = vec![0; length.expect("content length")];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}
