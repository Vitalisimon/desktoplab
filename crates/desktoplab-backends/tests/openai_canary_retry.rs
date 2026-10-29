use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use desktoplab_backends::{
    BackendModelCapabilities, ModelProtocolCertificationState, ModelToolProtocolKind,
    OpenAiCompatibleToolProtocolCanary,
};
use serde_json::{Value, json};
use xtask::check_logical_line_limit;

#[test]
fn truncated_reasoning_canary_retries_within_a_bounded_budget() {
    let (endpoint, server) = serve(vec![
        json!({"choices":[{"finish_reason":"length","message":{"reasoning":"unfinished"}}]}),
        json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"}}]}),
    ]);
    let caps = BackendModelCapabilities::reported(
        "backend.mlx-lm",
        "model.local",
        Some("test".into()),
        Some(32768),
        ["completion", "tools"],
    );
    let result = OpenAiCompatibleToolProtocolCanary::default().certify(
        &endpoint,
        &caps,
        ModelToolProtocolKind::ConstrainedJson,
        10,
    );
    let requests = server.join().unwrap();
    assert_eq!(result.state(), ModelProtocolCertificationState::Certified);
    assert_eq!(
        requests
            .iter()
            .map(|v| v["max_tokens"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![512, 1024]
    );
}

#[test]
fn successful_canary_does_not_retry() {
    let (endpoint, server) = serve(vec![
        json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"name\":\"desktoplab.list_files\",\"arguments\":{\"path\":\".\"}}"}}]}),
    ]);
    let caps = BackendModelCapabilities::reported(
        "backend.mlx-lm",
        "model.local",
        Some("test".into()),
        Some(32768),
        ["completion", "tools"],
    );
    let result = OpenAiCompatibleToolProtocolCanary::default().certify(
        &endpoint,
        &caps,
        ModelToolProtocolKind::ConstrainedJson,
        10,
    );
    assert_eq!(result.state(), ModelProtocolCertificationState::Certified);
    assert_eq!(server.join().unwrap().len(), 1);
}

fn serve(responses: Vec<Value>) -> (String, thread::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut requests = Vec::new();
        while requests.len() < responses.len() && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let len = headers
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + len {
                        requests
                            .push(serde_json::from_slice(&bytes[end + 4..end + 4 + len]).unwrap());
                        break;
                    }
                }
            }
            let body = responses[requests.len() - 1].to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
        requests
    });
    (format!("http://{address}"), handle)
}

#[test]
fn canary_retry_test_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-backends/tests/openai_canary_retry.rs",
        include_str!("openai_canary_retry.rs"),
        150,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-backends/src/openai_canary_retry.rs",
        include_str!("../src/openai_canary_retry.rs"),
        70,
    )
    .unwrap();
}
