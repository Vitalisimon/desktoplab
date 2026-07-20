use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiAuth, LocalAuthToken,
    VersionInfo,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn protected_routes_reject_non_loopback_host_headers() {
    let (address, handle) = authenticated_server();

    let response = read_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: desktoplab.example.com\r\nAuthorization: Bearer test-token\r\n\r\n",
    );

    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(
        response.contains("non-loopback host rejected"),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn protected_routes_allow_packaged_platform_and_explicit_dev_origins() {
    let (address, handle) = authenticated_server();

    let packaged = read_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: tauri://localhost\r\nAuthorization: Bearer test-token\r\n\r\n",
    );
    assert!(packaged.contains("200 OK"), "{packaged}");
    assert!(
        packaged.contains("Access-Control-Allow-Origin: tauri://localhost"),
        "{packaged}"
    );

    let windows_packaged = read_response(
        address,
        "GET /v1/setup/preview HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://tauri.localhost\r\nAuthorization: Bearer test-token\r\n\r\n",
    );
    assert!(windows_packaged.contains("200 OK"), "{windows_packaged}");
    assert!(
        windows_packaged.contains("Access-Control-Allow-Origin: http://tauri.localhost"),
        "{windows_packaged}"
    );

    let dev = read_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1:1420\r\nAuthorization: Bearer test-token\r\n\r\n",
    );
    assert!(dev.contains("200 OK"), "{dev}");
    assert!(
        dev.contains("Access-Control-Allow-Origin: http://127.0.0.1:1420"),
        "{dev}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn hostile_origin_never_opens_wildcard_cors() {
    let (address, handle) = authenticated_server();

    let response = read_response(
        address,
        "OPTIONS /v1/workspaces/open HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://evil.example\r\n\r\n",
    );

    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(
        !response.contains("Access-Control-Allow-Origin: *"),
        "{response}"
    );
    assert!(
        response.contains("Access-Control-Allow-Origin: null"),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn health_route_remains_usable_for_local_probes() {
    let (address, handle) = authenticated_server();

    let response = read_response(
        address,
        "GET /health HTTP/1.1\r\nHost: desktoplab.example.com\r\n\r\n",
    );

    assert!(response.contains("200 OK"), "{response}");
    assert!(
        response.contains("Access-Control-Allow-Origin: *"),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
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
