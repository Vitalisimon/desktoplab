use desktoplab_backend_services::{
    BackendEventScope, BackendEventStreamService, EventReplayRequest,
};
use desktoplab_control_plane::{LocalApiRouter, bind_default_local_api_server};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome, TerminalApproval,
    TerminalCommandRequest, TerminalToolExecutor, TerminalToolOutcome,
};
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::{thread, time::Duration};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
fn mutating_local_api_requires_auth_and_does_not_expose_wildcard_cors() {
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
        "{response}"
    );

    let preflight = read_response(
        address,
        "OPTIONS /v1/workspaces/open HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(preflight.contains("204 No Content"), "{preflight}");
    assert!(
        !preflight.contains("Access-Control-Allow-Origin: *"),
        "{preflight}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn request_body_self_approval_cannot_execute_terminal_commands() {
    let fixture = TempDir::new().expect("workspace fixture should exist");
    let workspace_root = fixture.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("workspace should exist");
    let mut router = LocalApiRouter::default();
    open_workspace(&mut router, &workspace_root.display().to_string());

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        r#"{"command":"printf should-not-run","cwd":"","approval":"approved","approvalRequired":true}"#,
    );

    assert_eq!(response["state"], "approval_required");
    assert!(response.get("events").is_none());
}

#[test]
fn approval_denial_prevents_terminal_execution() {
    let fixture = TempDir::new().expect("workspace fixture should exist");
    let workspace_root = fixture.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("workspace should exist");
    let mut router = LocalApiRouter::default();
    open_workspace(&mut router, &workspace_root.display().to_string());
    let approval_id = create_resolved_terminal_approval(&mut router, "deny", "printf denied");

    let response = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.workspace/terminal/commands",
        &format!(
            r#"{{"command":"printf denied","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(response["state"], "denied");
    assert!(response.get("events").is_none());
}

#[test]
fn sensitive_event_payloads_are_redacted_before_replay() {
    let mut stream = BackendEventStreamService::default();
    stream.publish_terminal_output("terminal.local", "stdout", "token=secret PASS", false);

    let replay = stream.replay(EventReplayRequest::new().scope(BackendEventScope::Terminal));

    assert!(replay.payloads()[0].contains("token=[REDACTED]"));
    assert!(!replay.payloads()[0].contains("token=secret"));
}

#[test]
#[cfg(unix)]
fn symlink_escapes_are_denied_for_filesystem_and_terminal_surfaces() {
    let fixture = TempDir::new().expect("workspace fixture should exist");
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(&outside).expect("outside dir should exist");
    fs::write(outside.join("secret.txt"), "do not overwrite").expect("outside file should write");
    symlink(
        outside.join("secret.txt"),
        workspace.join("linked-secret.txt"),
    )
    .unwrap();
    symlink(&outside, workspace.join("linked-outside")).unwrap();

    let mut filesystem =
        FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());
    let mut terminal = TerminalToolExecutor::new(
        &workspace,
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    assert_eq!(
        filesystem.read("linked-secret.txt"),
        FilesystemToolOutcome::Blocked("path_escape")
    );
    assert_eq!(
        filesystem.write(
            "linked-secret.txt",
            "overwritten",
            FilesystemApproval::Approved
        ),
        FilesystemToolOutcome::Blocked("path_escape")
    );
    assert_eq!(
        terminal.execute(
            TerminalCommandRequest::new("workspace.fixture", "pwd")
                .with_working_directory("linked-outside"),
            TerminalApproval::Approved,
        ),
        TerminalToolOutcome::Blocked("path_escape")
    );
}

#[test]
fn pre_packaging_security_gate_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/pre_packaging_security_gate.rs",
        include_str!("pre_packaging_security_gate.rs"),
        260,
    )
    .expect("pre-packaging security gate should stay focused");
}

fn open_workspace(router: &mut LocalApiRouter, path: &str) {
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
    router.set_host_memory_gb_for_test(32);
    route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(router, "POST", "/v1/setup/complete", "{}");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &format!(r#"{{"path":"{path}"}}"#),
    );
}

fn create_resolved_terminal_approval(
    router: &mut LocalApiRouter,
    resolution: &str,
    command: &str,
) -> String {
    let created = route_json(
        router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.local","action":"terminal.command","operationId":"workspace.workspace:terminal.local","payload":{{"command":"{command}","cwd":""}}}}"#
        ),
    );
    let approval_id = created["approvalId"].as_str().unwrap().to_string();
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        &format!(r#"{{"resolution":"{resolution}"}}"#),
    );
    approval_id
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
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
