use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiRouter, VersionInfo,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn slow_initial_and_post_approval_model_turns_do_not_hold_router_lock() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"async.md","content":"outside lock\n"}}"#,
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"async.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Created async.md.","outcome":"changed","evidenceCallIds":["write-1","read-1"]}}"#,
    ]);
    router.set_agent_model_delay_for_test(Duration::from_millis(800));
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();

    let started = Instant::now();
    let created = request_json(
        address,
        &post_json_request(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Create async.md."}}"#
            ),
        ),
    );
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(created["state"], "running");
    assert_diagnostics_responsive(address);

    let blocked = wait_for_session_state(address, &created["sessionId"], "blocked");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    let approval_started = Instant::now();
    request_json(
        address,
        &post_json_request(
            &format!("/v1/approvals/{approval_id}/resolve"),
            r#"{"resolution":"approve"}"#,
        ),
    );
    assert!(approval_started.elapsed() < Duration::from_millis(500));
    wait_for_path(&workspace.path().join("async.md"));
    thread::sleep(Duration::from_millis(100));
    assert_diagnostics_responsive(address);

    let completed = wait_for_session_state(address, &created["sessionId"], "completed");
    assert_eq!(completed["summary"], "Created async.md.");
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("async.md")).unwrap(),
        "outside lock\n"
    );
    handle.shutdown().expect("server should stop");
}

#[test]
fn cancel_token_stops_an_inflight_model_turn_without_late_completion() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Must not appear.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);
    router.set_agent_model_delay_for_test(Duration::from_secs(2));
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let created = request_json(
        address,
        &post_json_request(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Wait","stream":true}}"#
            ),
        ),
    );
    let session_id = created["sessionId"].as_str().unwrap();
    thread::sleep(Duration::from_millis(100));

    let cancelled = request_json(
        address,
        &post_json_request(
            &format!("/v1/sessions/{session_id}/control"),
            r#"{"action":"cancel"}"#,
        ),
    );

    assert_eq!(cancelled["state"], "cancelled");
    thread::sleep(Duration::from_millis(150));
    let persisted = wait_for_session_state(address, &created["sessionId"], "cancelled");
    assert!(persisted["summary"].is_null());
    handle.shutdown().expect("server should stop");
}

#[test]
fn worker_events_are_durable_without_a_followup_mutating_request() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("repo");
    std::fs::create_dir(&workspace).unwrap();
    create_repo(&workspace);
    let database = fixture.path().join("desktoplab.sqlite");
    let router = LocalApiRouter::with_storage_path(&database).unwrap();
    let (mut router, workspace_id) = configure_router(router, &workspace);
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Durable completion.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let created = request_json(
        address,
        &post_json_request(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Answer briefly.","stream":true}}"#
            ),
        ),
    );
    wait_for_session_state(address, &created["sessionId"], "completed");
    handle.shutdown().unwrap();

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    let replay = route_json(&mut restarted, "GET", "/v1/events/replay", "");
    assert!(
        replay["frames"]
            .to_string()
            .contains("agent.stream.completed"),
        "{replay}"
    );
}

#[test]
fn asynchronous_worker_recovers_one_invalid_tool_name() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"read_file","arguments":{"path":"README.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Async protocol recovered.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);
    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let created = request_json(
        address,
        &post_json_request(
            "/v1/sessions",
            &format!(
                r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Answer briefly."}}"#
            ),
        ),
    );

    let completed = wait_for_session_state(address, &created["sessionId"], "completed");

    assert_eq!(completed["summary"], "Async protocol recovered.");
    handle.shutdown().expect("server should stop");
}

#[test]
fn model_concurrency_test_stays_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-control-plane/tests/local_api_agent_model_concurrency.rs",
            include_str!("local_api_agent_model_concurrency.rs"),
            320,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_model_jobs.rs",
            include_str!("../src/router/agent_model_jobs.rs"),
            320,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_model_constrained.rs",
            include_str!("../src/router/agent_model_constrained.rs"),
            120,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_model_local.rs",
            include_str!("../src/router/agent_model_local.rs"),
            180,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_iterative_resume.rs",
            include_str!("../src/router/agent_iterative_resume.rs"),
            220,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit)
            .expect("agent model concurrency source should stay focused");
    }
}

fn assert_diagnostics_responsive(address: SocketAddr) {
    let started = Instant::now();
    let response = read_response(
        address,
        "GET /v1/diagnostics HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(response.contains("200 OK"), "{response}");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "diagnostics waited for model inference: {:?}",
        started.elapsed()
    );
}

fn wait_for_session_state(address: SocketAddr, session_id: &Value, state: &str) -> Value {
    let session_id = session_id.as_str().unwrap();
    for _ in 0..100 {
        let payload = request_json(
            address,
            &format!("GET /v1/sessions HTTP/1.1\r\nHost: localhost\r\n\r\n"),
        );
        if let Some(session) = payload["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|item| item["sessionId"] == session_id))
            && session["state"] == state
        {
            return session.clone();
        }
        thread::sleep(Duration::from_millis(30));
    }
    panic!("session {session_id} did not reach {state}");
}

fn wait_for_path(path: &std::path::Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("approved write was not executed");
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

fn request_json(address: SocketAddr, request: &str) -> Value {
    let response = read_response(address, request);
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("{}");
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid response: {response}"))
}

fn post_json_request(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn read_response(address: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    configure_router(LocalApiRouter::default(), workspace)
}

fn configure_router(
    mut router: LocalApiRouter,
    workspace: &std::path::Path,
) -> (LocalApiRouter, String) {
    router.set_host_memory_gb_for_test(32);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    serde_json::from_str(response.body()).unwrap()
}

fn create_repo(path: &std::path::Path) {
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .unwrap();
    assert!(status.success());
}
