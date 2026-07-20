use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiRouter, VersionInfo,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn http_approval_returns_before_agent_execution_and_worker_resumes_it() {
    let workspace = TempDir::new().expect("temp workspace");
    create_repo(workspace.path());
    let command_started = workspace.path().join("command-started");
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_agent_backend_sequence_for_test([
        &terminal_action(&slow_command(&command_started)),
        r#"{"name":"desktoplab.complete","arguments":{"message":"Slow command completed."}}"#,
    ]);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"run the slow validation"}}"#
        ),
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .expect("pending approval id")
        .to_string();
    let session_id = blocked["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();

    let server = server_with_router(router);
    let address = server.local_addr();
    let handle = server.spawn();
    let started = Instant::now();
    let resolved = read_response(
        address,
        &post_json_request(
            &format!("/v1/approvals/{approval_id}/resolve"),
            r#"{"resolution":"approve"}"#,
        ),
    );

    assert!(resolved.contains("200 OK"), "{resolved}");
    assert!(resolved.contains(r#""state":"approved""#), "{resolved}");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "approval response waited for agent execution: {:?}",
        started.elapsed()
    );

    wait_for_path(&command_started);
    let diagnostics_started = Instant::now();
    let diagnostics = read_response(
        address,
        "GET /v1/diagnostics HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(diagnostics.contains("200 OK"), "{diagnostics}");
    assert!(
        diagnostics_started.elapsed() < Duration::from_millis(500),
        "diagnostics waited for the approved tool: {:?}",
        diagnostics_started.elapsed()
    );

    let completed = wait_for_completed_session(address, &session_id);
    assert_eq!(completed["state"], "completed", "{completed}");
    assert!(completed["pendingApprovals"].as_array().unwrap().is_empty());
    handle.shutdown().expect("server should stop");
}

#[test]
fn deferred_approval_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_deferred_approval.rs",
        include_str!("local_api_agent_deferred_approval.rs"),
        240,
    )
    .expect("deferred approval test should stay focused");
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b    5.2 GB"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b    5.2 GB"}"#,
    );
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    let workspace_id = opened["workspaceId"]
        .as_str()
        .expect("workspace id")
        .to_string();
    (router, workspace_id)
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
    .expect("server should bind")
}

fn wait_for_completed_session(address: SocketAddr, session_id: &str) -> Value {
    for _ in 0..20 {
        let response = read_response(
            address,
            &format!(
                "GET /v1/sessions?session_id={session_id} HTTP/1.1\r\nHost: localhost\r\n\r\n"
            ),
        );
        let body = response.split("\r\n\r\n").nth(1).unwrap_or("{}");
        let payload: Value = serde_json::from_str(body).expect("session response json");
        let session = payload["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|item| item["sessionId"] == session_id))
            .cloned()
            .unwrap_or(Value::Null);
        if session["state"] == "completed" {
            return session;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("agent session did not complete");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    serde_json::from_str(response.body()).expect("route response json")
}

fn post_json_request(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn read_response(address: SocketAddr, request: &str) -> String {
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

fn terminal_action(command: &str) -> String {
    serde_json::json!({
        "name":"desktoplab.run_terminal",
        "arguments":{"command":command,"reason":"prove deferred approval execution"}
    })
    .to_string()
}

fn wait_for_path(path: &std::path::Path) {
    for _ in 0..50 {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("approved command did not start");
}

#[cfg(unix)]
fn slow_command(marker: &std::path::Path) -> String {
    format!("touch '{}'; sleep 1", marker.display())
}

#[cfg(windows)]
fn slow_command(marker: &std::path::Path) -> String {
    format!(
        "powershell -NoProfile -Command \"New-Item -ItemType File -Force -LiteralPath '{}'; Start-Sleep -Milliseconds 1000\"",
        marker.display()
    )
}

fn create_repo(path: &std::path::Path) {
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
}
