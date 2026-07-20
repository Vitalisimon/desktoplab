use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, VersionInfo,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn local_api_reads_valid_request_body_beyond_single_socket_buffer() {
    let (address, handle) = server();
    let padding = "x".repeat(2_700);
    let body = format!(
        r#"{{"notes":"{padding}","runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}}"#
    );

    let response = read_response(address, &post_json_request("/v1/setup/accept", &body));

    assert!(response.contains("200 OK"), "{response}");
    assert!(
        response.contains(r#""modelId":"model.gemma4-12b-q4""#),
        "{response}"
    );
    assert!(
        response.contains(r#""runtimeId":"runtime.ollama""#),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn local_api_accepts_the_declared_text_attachment_envelope() {
    let (address, handle) = server();
    let body = format!(
        r#"{{"notes":"{}","runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}}"#,
        "x".repeat(600 * 1024)
    );

    let response = read_response(address, &post_json_request("/v1/setup/accept", &body));

    assert!(response.contains("200 OK"), "{response}");
    handle.shutdown().expect("server should stop");
}

#[test]
fn local_api_rejects_oversized_request_body_explicitly() {
    let (address, handle) = server();
    let request = post_declared_length("/v1/setup/accept", 4 * 1024 * 1024 + 1);

    let response = read_response(address, &request);

    assert!(response.contains("413 Payload Too Large"), "{response}");
    assert!(
        response.contains(r#""code":"PAYLOAD_TOO_LARGE""#),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

fn post_declared_length(path: &str, content_length: usize) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\n\r\n"
    )
}

#[test]
fn local_api_http_body_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_http_body.rs",
        include_str!("local_api_http_body.rs"),
        120,
    )
    .expect("local API HTTP body test should stay focused");
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/http/request_body.rs",
        include_str!("../src/http/request_body.rs"),
        120,
    )
    .expect("local API HTTP body parser should stay focused");
}

fn server() -> (SocketAddr, desktoplab_control_plane::HttpServerHandle) {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    let server =
        ControlPlaneHttpServer::bind(HttpServerConfig::loopback(0).unwrap(), control_plane)
            .expect("server should bind");
    let address = server.local_addr();
    (address, server.spawn())
}

fn post_json_request(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn read_response(address: SocketAddr, request: &str) -> String {
    let mut last_response = String::new();
    for _ in 0..5 {
        let mut stream = TcpStream::connect(address).expect("server should accept connection");
        stream
            .write_all(request.as_bytes())
            .expect("request should write");
        let mut response = String::new();
        if stream.read_to_string(&mut response).is_ok() && !response.is_empty() {
            return response;
        }
        last_response = response;
        thread::sleep(Duration::from_millis(10));
    }
    last_response
}
