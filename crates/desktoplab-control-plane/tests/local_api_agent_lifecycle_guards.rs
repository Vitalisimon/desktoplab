use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiRouter, VersionInfo,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn paused_inflight_model_turn_cannot_complete_until_explicit_resume() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    let completion = r#"{"tool":"desktoplab.complete","arguments":{"message":"Resumed safely.","outcome":"answered","evidenceCallIds":[]}}"#;
    router.complete_native_iterative_backend_sequence_for_test([completion, completion]);
    router.set_agent_model_delay_for_test(Duration::from_millis(500));
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let created = request_json(
        address,
        &post(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Wait"}}"#
            ),
        ),
    );
    let session_id = created["sessionId"].as_str().unwrap();
    thread::sleep(Duration::from_millis(50));

    let paused = request_json(
        address,
        &post(
            &format!("/v1/sessions/{session_id}/control"),
            r#"{"action":"pause"}"#,
        ),
    );
    assert_eq!(paused["state"], "paused");
    thread::sleep(Duration::from_millis(650));
    assert_eq!(session(address, session_id)["state"], "paused");

    request_json(
        address,
        &post(
            &format!("/v1/sessions/{session_id}/control"),
            r#"{"action":"resume"}"#,
        ),
    );
    assert_eq!(
        wait_for_state(address, session_id, "completed")["summary"],
        "Resumed safely."
    );
    handle.shutdown().unwrap();
}

#[test]
fn cancelled_session_cannot_be_resumed_through_the_control_api() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.complete","arguments":{"message":"late","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);
    router.set_agent_model_delay_for_test(Duration::from_secs(1));
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let created = request_json(
        address,
        &post(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Wait"}}"#
            ),
        ),
    );
    let session_id = created["sessionId"].as_str().unwrap();
    request_json(
        address,
        &post(
            &format!("/v1/sessions/{session_id}/control"),
            r#"{"action":"cancel"}"#,
        ),
    );

    let response = read_response(
        address,
        &post(
            &format!("/v1/sessions/{session_id}/control"),
            r#"{"action":"resume"}"#,
        ),
    );
    assert!(response.contains("400 Bad Request"), "{response}");
    assert_eq!(session(address, session_id)["state"], "cancelled");
    handle.shutdown().unwrap();
}

fn ready_router(path: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    route(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&path),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn server_with_router(router: LocalApiRouter) -> ControlPlaneHttpServer {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane.lock().unwrap().mark_ready();
    ControlPlaneHttpServer::bind_with_router(
        HttpServerConfig::loopback(0).unwrap(),
        control_plane,
        router,
    )
    .unwrap()
}

fn wait_for_state(address: SocketAddr, session_id: &str, expected: &str) -> Value {
    for _ in 0..100 {
        let value = session(address, session_id);
        if value["state"] == expected {
            return value;
        }
        thread::sleep(Duration::from_millis(30));
    }
    panic!("session did not reach {expected}");
}

fn session(address: SocketAddr, session_id: &str) -> Value {
    request_json(
        address,
        "GET /v1/sessions HTTP/1.1\r\nHost: localhost\r\n\r\n",
    )["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|session| session["sessionId"] == session_id)
        .unwrap()
        .clone()
}

fn route(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    serde_json::from_str(router.route(method, path, body).unwrap().body()).unwrap()
}

fn request_json(address: SocketAddr, request: &str) -> Value {
    let response = read_response(address, request);
    serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}

fn read_response(address: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn post(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn create_repo(path: &std::path::Path) {
    assert!(
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn lifecycle_guard_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_lifecycle_guards.rs",
        include_str!("local_api_agent_lifecycle_guards.rs"),
        230,
    )
    .unwrap();
}
