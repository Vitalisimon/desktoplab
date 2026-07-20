use desktoplab_desktop_lib::{PackagedLocalApi, PackagedLocalApiConfig};
use std::io::{Read, Write};
use std::net::TcpStream;
use tempfile::TempDir;

#[test]
fn packaged_boot_writes_redacted_discovery_and_requires_auth() {
    let app_data = TempDir::new().expect("app data fixture should exist");
    let discovery_dir = TempDir::new().expect("discovery fixture should exist");
    let discovery_path = discovery_dir.path().join("local-api-discovery.json");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(app_data.path())
            .with_discovery_path(&discovery_path),
    )
    .expect("packaged local api should start");

    let unauthorized = read_response(
        api.bound_addr(),
        "GET /v1/app/state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(unauthorized.contains("401 Unauthorized"), "{unauthorized}");

    let discovery = std::fs::read_to_string(&discovery_path).expect("discovery should persist");
    assert!(discovery.contains(api.base_url()), "{discovery}");
    assert!(discovery.contains(r#""schemaVersion":1"#));
    assert!(discovery.contains(r#""tokenRedacted":"[REDACTED_LOCAL_API_TOKEN]""#));
    assert!(!discovery.contains(api.auth_token()));

    api.shutdown().expect("packaged local api should stop");
    assert!(!discovery_path.exists());
}

#[test]
fn packaged_api_persists_product_state_across_restart_without_host_runtime_dependencies() {
    let app_data = TempDir::new().expect("app data fixture should exist");
    {
        let api = PackagedLocalApi::start(
            PackagedLocalApiConfig::random_loopback().with_app_data_dir(app_data.path()),
        )
        .expect("packaged local api should start");
        let update = request(
            &api,
            "POST /v1/approval-modes/default HTTP/1.1",
            r#"{"mode":"full_access"}"#,
        );
        assert!(update.contains("200 OK"), "{update}");
        assert!(update.contains(r#""defaultMode":"full_access""#), "{update}");
    }

    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback().with_app_data_dir(app_data.path()),
    )
    .expect("packaged local api should restart");
    let modes = request(&api, "GET /v1/approval-modes HTTP/1.1", "");

    assert!(modes.contains(r#""defaultMode":"full_access""#), "{modes}");
}

fn request(api: &PackagedLocalApi, line: &str, body: &str) -> String {
    read_response(
        api.bound_addr(),
        &format!(
            "{line}\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\nContent-Length: {}\r\n\r\n{body}",
            api.auth_token(),
            body.len()
        ),
    )
}

fn read_response(address: std::net::SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("server should accept connection");
    stream
        .write_all(request.as_bytes())
        .expect("request should write");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("response should read");
    response
}
