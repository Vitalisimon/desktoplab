use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use desktoplab_backends::{BackendModelInventory, BackendPrompt, OllamaExecutionBackend};

#[test]
fn ollama_request_uses_the_prompt_timeout_instead_of_the_client_default() {
    let (endpoint, server) = delayed_server(r#"{"message":{"content":"late"}}"#);
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model"]));
    let prompt = BackendPrompt::new("model", "hello").with_request_timeout_seconds(1);
    let started = Instant::now();

    let error = backend
        .execute_chat(&endpoint, &prompt)
        .expect_err("one-second policy must cancel the delayed response");

    server.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(error.starts_with("ollama_request_failed:"), "{error}");
}

#[test]
fn ollama_stream_uses_the_same_prompt_timeout() {
    let (endpoint, server) = delayed_server("{\"message\":{\"content\":\"late\"},\"done\":true}\n");
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&["model"]));
    let prompt = BackendPrompt::new("model", "hello").with_request_timeout_seconds(1);
    let started = Instant::now();

    let error = backend
        .execute_chat_stream(&endpoint, &prompt, &AtomicBool::new(false), |_| {})
        .expect_err("one-second policy must cancel the delayed stream");

    server.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(error.starts_with("ollama_request_failed:"), "{error}");
}

fn delayed_server(body: &'static str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test endpoint");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept Ollama request");
        let mut request = [0_u8; 4096];
        stream.read(&mut request).expect("read request");
        thread::sleep(Duration::from_millis(1_200));
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
    });
    (endpoint, server)
}
