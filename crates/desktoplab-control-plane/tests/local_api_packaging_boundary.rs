use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, DiscoveryPermissionState, HttpServerConfig, LocalApiAuth,
    LocalApiDiscoveryDocument, LocalApiDiscoveryPath, LocalApiDiscoveryWriter, LocalAuthToken,
    VersionInfo, bind_default_local_api_server,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn packaged_api_uses_random_loopback_and_auth_for_sensitive_routes() {
    let server = bind_default_local_api_server(0).expect("packaged api should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    assert!(address.ip().is_loopback());
    assert_ne!(address.port(), 0);
    assert_response(
        address,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
    );
    for request in [
        "GET /v1/app/state HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "POST /v1/runtimes/runtime.ollama/install HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}",
        "POST /v1/models/model.gemma4-12b-q4/download HTTP/1.1\r\nHost: localhost\r\nContent-Length: 30\r\n\r\n{\"runtimeId\":\"runtime.ollama\"}",
    ] {
        assert_response(address, request, "401 Unauthorized");
    }

    handle.shutdown().expect("server should stop");
}

#[test]
fn packaged_api_rejects_non_loopback_origin_even_with_token() {
    let (address, handle) = authenticated_server();
    let response = read_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: desktoplab.example.com\r\nOrigin: https://evil.example\r\nAuthorization: Bearer test-token\r\n\r\n",
    );

    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(
        !response.contains("Access-Control-Allow-Origin: *"),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
#[cfg(unix)]
fn packaged_discovery_is_redacted_user_only_and_bootstrap_safe() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "desktoplab")
        .expect("discovery path should build");
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document =
        LocalApiDiscoveryDocument::new("http://127.0.0.1:48123", 42, 1_719_000_000, &token)
            .expect("discovery document should be valid");

    LocalApiDiscoveryWriter::write(&path, &document).expect("discovery should write");
    let persisted = std::fs::read_to_string(path.as_path()).expect("discovery should persist");
    let mode = std::fs::metadata(path.as_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(mode, 0o600);
    assert!(!persisted.contains("raw-secret-token"));
    assert_eq!(
        LocalApiDiscoveryWriter::verify_permissions(&path).unwrap(),
        DiscoveryPermissionState::UserOnly
    );
    assert!(
        LocalApiDiscoveryWriter::verify_permissions(&path)
            .unwrap()
            .allows_packaged_bootstrap()
    );
}

#[test]
fn local_api_packaging_boundary_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_packaging_boundary.rs",
        include_str!("local_api_packaging_boundary.rs"),
        180,
    )
    .expect("packaging boundary test should stay focused");
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

fn assert_response(address: SocketAddr, request: &str, status: &str) {
    let response = read_response(address, request);
    assert!(response.contains(status), "{response}");
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
