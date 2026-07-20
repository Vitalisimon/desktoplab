use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiAuth, LocalAuthToken,
    VersionInfo,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use xtask::check_logical_line_limit;

#[test]
fn protected_endpoints_reject_missing_token() {
    let (address, handle) = authenticated_server();

    assert_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "401 Unauthorized",
        r#"{"code":"UNAUTHORIZED","message":"local auth token is missing"}"#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn protected_endpoints_reject_invalid_token() {
    let (address, handle) = authenticated_server();

    assert_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrong\r\n\r\n",
        "401 Unauthorized",
        r#"{"code":"UNAUTHORIZED","message":"local auth token is invalid"}"#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn protected_endpoints_accept_explicit_test_token() {
    let (address, handle) = authenticated_server();

    assert_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-token\r\n\r\n",
        "200 OK",
        r#"{"productVersion":"0.1.0","apiVersion":"v1"}"#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn auth_tokens_are_redacted_for_logs_and_events() {
    let token = LocalAuthToken::explicit_for_test("test-token");

    assert_eq!(token.redacted(), "[REDACTED_LOCAL_API_TOKEN]");
    assert!(!token.redacted().contains("test-token"));
}

#[test]
fn desktop_session_tokens_do_not_expose_process_or_time_material() {
    let token = LocalAuthToken::for_desktop_session();
    let value = token.as_str();

    assert!(value.len() >= 64);
    assert!(!value.contains(&std::process::id().to_string()));
    assert!(!value.starts_with("desktoplab-local-api-"));
}

#[test]
fn local_api_auth_source_stays_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/auth.rs",
        include_str!("../src/auth.rs"),
        250,
    )
    .expect("control-plane auth source should stay below the line-count guard");
}

fn authenticated_server() -> (SocketAddr, desktoplab_control_plane::HttpServerHandle) {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    let server = ControlPlaneHttpServer::bind(
        HttpServerConfig::loopback(0)
            .unwrap()
            .with_auth(LocalApiAuth::required(LocalAuthToken::explicit_for_test(
                "test-token",
            ))),
        control_plane,
    )
    .expect("server should bind");
    let address = server.local_addr();
    (address, server.spawn())
}

fn assert_response(address: SocketAddr, request: &str, status: &str, body: &str) {
    let response = read_response(address, request);

    assert!(response.contains(status), "{response}");
    assert!(response.ends_with(body), "{response}");
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
