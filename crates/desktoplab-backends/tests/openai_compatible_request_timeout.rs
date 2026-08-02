use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, LmStudioExecutionBackend, LocalEndpoint,
    OpenAiCompatibleLocalExecutionBackend,
};
use xtask::check_logical_line_limit;

#[test]
fn lm_studio_chat_uses_the_prompt_timeout() {
    let backend = lm_studio(delayed_json_server(chat_body("late")));
    assert_timeout("lm_studio_request_failed:", || {
        backend.execute_chat(&timed_prompt())
    });
}

#[test]
fn mlx_constrained_chat_uses_the_prompt_timeout() {
    let backend = lm_studio(delayed_json_server(chat_body(
        r#"{\"name\":\"desktoplab.complete\",\"arguments\":{}}"#,
    )));
    assert_timeout("mlx_request_failed:", || {
        backend.execute_constrained_chat(&timed_prompt())
    });
}

#[test]
fn lm_studio_stream_uses_the_prompt_timeout() {
    let backend = lm_studio(delayed_sse_server());
    assert_timeout("openai_compatible_stream_request_failed:", || {
        backend.execute_chat_stream(&timed_prompt(), &AtomicBool::new(false), |_| {})
    });
}

#[test]
fn generic_openai_compatible_chat_uses_the_prompt_timeout() {
    let endpoint = delayed_json_server(chat_body("late"));
    let backend = OpenAiCompatibleLocalExecutionBackend::new(
        "backend.local",
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["model"]),
    );
    assert_timeout("openai_compatible_local_request_failed:", || {
        backend.execute_chat(&timed_prompt())
    });
}

#[test]
fn openai_compatible_timeout_sources_stay_focused() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/openai_compatible_http.rs",
            include_str!("../src/openai_compatible_http.rs"),
            30,
        ),
        (
            "crates/desktoplab-backends/tests/openai_compatible_request_timeout.rs",
            include_str!("openai_compatible_request_timeout.rs"),
            130,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines).expect("timeout source stays focused");
    }
}

fn assert_timeout(prefix: &str, execute: impl FnOnce() -> Result<String, String>) {
    let started = Instant::now();
    let error = execute().expect_err("one-second prompt policy must stop the delayed response");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(error.starts_with(prefix), "{error}");
}

fn timed_prompt() -> BackendPrompt {
    BackendPrompt::new("model", "hello").with_request_timeout_seconds(1)
}

fn lm_studio(endpoint: String) -> LmStudioExecutionBackend {
    LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&["model"]),
    )
}

fn chat_body(content: &str) -> String {
    serde_json::json!({"choices":[{"message":{"content":content}}]}).to_string()
}

fn delayed_json_server(body: String) -> String {
    delayed_server("application/json", body)
}

fn delayed_sse_server() -> String {
    delayed_server(
        "text/event-stream",
        "data: {\"choices\":[{\"delta\":{\"content\":\"late\"}}]}\n\ndata: [DONE]\n\n".to_string(),
    )
}

fn delayed_server(content_type: &'static str, body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test endpoint");
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept local request");
        let mut request = [0_u8; 8192];
        stream.read(&mut request).expect("read request");
        thread::sleep(Duration::from_millis(1_200));
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len(),
        );
    });
    format!("http://{address}")
}
