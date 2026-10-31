use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use desktoplab_backends::{BackendModelInventory, BackendPrompt, OllamaExecutionBackend};

use super::retry_inference_timeout;

#[test]
fn real_request_timeout_recovers_once_without_executing_a_tool() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let started = Instant::now();
        let mut count = 0;
        while started.elapsed() < Duration::from_secs(5) && count < 2 {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 4096];
                    stream.read(&mut request).unwrap();
                    count += 1;
                    if count == 1 {
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(1300));
                            drop(stream);
                        });
                    } else {
                        let body = r#"{"message":{"content":"recovered"}}"#;
                        write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}", body.len(), body).unwrap();
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("listener failed: {error}"),
            }
        }
        count
    });
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model"]));
    let prompt = BackendPrompt::new("model", "task").with_request_timeout_seconds(1);
    let output = retry_inference_timeout(&AtomicBool::new(false), || {
        backend.execute_chat(&endpoint, &prompt)
    });
    assert_eq!(output.unwrap(), "recovered");
    assert_eq!(server.join().unwrap(), 2);
}

#[test]
fn streamed_partial_output_is_not_replayed_after_a_read_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let started = Instant::now();
        let mut count = 0;
        while started.elapsed() < Duration::from_secs(3) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 4096];
                    stream.read(&mut request).unwrap();
                    count += 1;
                    thread::spawn(move || {
                        let body = "{\"message\":{\"content\":\"once\"},\"done\":false}\n";
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\ncontent-type: application/x-ndjson\r\n\r\n{body}"
                        )
                        .unwrap();
                        stream.flush().unwrap();
                        thread::sleep(Duration::from_millis(1500));
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("listener failed: {error}"),
            }
        }
        count
    });
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model"]));
    let prompt = BackendPrompt::new("model", "task").with_request_timeout_seconds(1);
    let cancellation = AtomicBool::new(false);
    let mut output = String::new();
    let result = retry_inference_timeout(&cancellation, || {
        backend.execute_chat_stream(&endpoint, &prompt, &cancellation, |delta| {
            output.push_str(delta)
        })
    });
    assert!(
        result
            .unwrap_err()
            .starts_with("ollama_stream_read_failed:")
    );
    assert_eq!(output, "once");
    assert_eq!(server.join().unwrap(), 1);
}

#[test]
fn timeout_integration_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_ollama_timeout_integration_tests.rs",
        include_str!("agent_ollama_timeout_integration_tests.rs"),
        130,
    )
    .expect("timeout integration test should stay focused");
}
