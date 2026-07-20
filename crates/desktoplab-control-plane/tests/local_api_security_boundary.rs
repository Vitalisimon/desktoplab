use desktoplab_control_plane::{bind_default_local_api_server, bind_unsafe_dev_local_api_server};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

#[test]
fn default_local_api_rejects_unauthenticated_mutations() {
    let server = bind_default_local_api_server(0).expect("default local api should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    let response = read_response(
        address,
        "POST /v1/workspaces/open HTTP/1.1\r\nHost: localhost\r\nContent-Length: 27\r\n\r\n{\"path\":\"/repo/desktoplab\"}",
    );

    assert!(response.contains("401 Unauthorized"), "{response}");
    assert!(
        !response.contains("Access-Control-Allow-Origin: *"),
        "mutating routes must not expose wildcard CORS: {response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn unsafe_dev_server_is_explicitly_named_and_allows_local_manual_probes() {
    let server = bind_unsafe_dev_local_api_server(0).expect("unsafe dev local api should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    let response = read_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );

    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains(r#""apiVersion":"v1""#), "{response}");

    handle.shutdown().expect("server should stop");
}

#[test]
fn security_boundary_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_security_boundary.rs",
        include_str!("local_api_security_boundary.rs"),
        180,
    )
    .expect("local api security boundary test should stay focused");
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
