use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiAuth, LocalAuthToken,
    VersionInfo,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

fn request(address: SocketAddr, line: &str, headers: &str) -> String {
    let mut socket = TcpStream::connect(address).unwrap();
    write!(
        socket,
        "{line}\r\nHost: localhost\r\n{headers}Connection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn protected_paths_apply_authentication_and_origin_to_all_valid_separators() {
    let control = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "fixture", "v1",
    ))));
    let auth = LocalApiAuth::required(LocalAuthToken::explicit_for_test("fixture-token"));
    let server = ControlPlaneHttpServer::bind(
        HttpServerConfig::loopback(0).unwrap().with_auth(auth),
        control,
    )
    .unwrap();
    let address = server.local_addr();
    let handle = server.spawn();
    for separator in [" ", "\t", " \t"] {
        let line = format!("GET{separator}/v1/app/state HTTP/1.1");
        assert!(request(address, &line, "").contains("401 Unauthorized"));
        assert!(
            request(address, &line, "Authorization: Bearer wrong\r\n").contains("401 Unauthorized")
        );
        assert!(
            request(address, &line, "Origin: https://untrusted.invalid\r\n")
                .contains("403 Forbidden")
        );
        assert!(
            request(address, &line, "Authorization: Bearer fixture-token\r\n").contains("200 OK")
        );
    }
    handle.shutdown().unwrap();
}

#[test]
fn authentication_regression_test_stays_focused() {
    xtask::check_logical_line_limit(
        "tests/local_api_auth_whitespace.rs",
        include_str!("local_api_auth_whitespace.rs"),
        100,
    )
    .unwrap();
}
