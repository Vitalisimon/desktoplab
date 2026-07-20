use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LifecycleState, LocalApiRouter,
    VersionInfo,
};
use serde_json::Value;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn server_binds_only_to_loopback() {
    assert!(HttpServerConfig::loopback(0).is_ok());
    assert!(HttpServerConfig::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0,)).is_err());
}

#[test]
fn health_readiness_and_version_return_stable_json() {
    let server = test_router_server();
    let address = server.local_addr();
    let handle = server.spawn();

    assert_response(
        address,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"status":"healthy"}"#,
    );
    assert_response(
        address,
        "GET /v1/readiness HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"state":"ready"}"#,
    );
    assert_response(
        address,
        "GET /v1/version HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"productVersion":"0.1.0","apiVersion":"v1"}"#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn public_health_stays_available_while_another_client_is_slow() {
    let server = test_router_server();
    let address = server.local_addr();
    let handle = server.spawn();
    let mut slow_stream = TcpStream::connect(address).expect("server should accept connection");
    slow_stream
        .write_all(
            b"POST /v1/workspaces/open HTTP/1.1\r\nHost: localhost\r\nContent-Length: 128\r\n\r\n",
        )
        .expect("partial request should write");

    let started = Instant::now();
    assert_response(
        address,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"status":"healthy"}"#,
    );

    assert!(
        started.elapsed() < Duration::from_millis(750),
        "health should not wait for a slow workspace request"
    );
    drop(slow_stream);
    handle.shutdown().expect("server should stop");
}

#[test]
fn unknown_routes_return_structured_error_json() {
    let server = ControlPlaneHttpServer::bind(
        HttpServerConfig::loopback(0).unwrap(),
        Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
            "0.1.0", "v1",
        )))),
    )
    .expect("server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    assert_response(
        address,
        "GET /missing HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "404 Not Found",
        r#"{"code":"NOT_FOUND","message":"route not found"}"#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn shutdown_endpoint_changes_lifecycle_state() {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    let server = ControlPlaneHttpServer::bind(
        HttpServerConfig::loopback(0).unwrap(),
        control_plane.clone(),
    )
    .expect("server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    assert_response(
        address,
        "POST /v1/shutdown HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"shutdown":"graceful"}"#,
    );

    assert_eq!(
        control_plane.lock().unwrap().lifecycle_state(),
        LifecycleState::Stopping
    );
    handle.join().expect("server should stop after shutdown");
}

#[test]
fn product_routes_are_served_by_local_api_router() {
    let fixture = workspace_fixture();
    let workspace_root = fixture.path().join("desktoplab");
    fs::create_dir_all(&workspace_root).expect("workspace directory should exist");
    init_git_repo(&workspace_root);
    let server = test_router_server();
    let address = server.local_addr();
    let handle = server.spawn();

    assert_response_contains(
        address,
        "GET /v1/setup/preview HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#""runtimeRecommendations""#,
    );
    complete_setup(address);
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&workspace_root),
        ),
        "200 OK",
        r#""workspaceId":"workspace.desktoplab""#,
    );
    assert_response_contains(
        address,
        "GET /v1/agent/workspace?workspace_id=workspace.desktoplab HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#""source":"service_backed""#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn workspace_file_routes_return_bounded_tree_and_safe_previews() {
    let fixture = workspace_fixture();
    let workspace_root = fixture.path().join("desktoplab_fixture");
    fs::create_dir_all(workspace_root.join("src")).expect("source directory should exist");
    fs::write(
        workspace_root.join("src/main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\nAPI_KEY=secret-value\n",
    )
    .expect("source file should write");
    fs::write(workspace_root.join(".env"), "TOKEN=do-not-leak").expect("env file should write");
    init_git_repo(&workspace_root);

    let server = test_router_server();
    let address = server.local_addr();
    let handle = server.spawn();

    complete_setup(address);
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&workspace_root),
        ),
        "200 OK",
        r#""workspaceId":"workspace.desktoplab_fixture""#,
    );

    assert_response_contains(
        address,
        "GET /v1/workspaces/workspace.desktoplab_fixture/files HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#""path":"src/main.rs""#,
    );

    let preview = read_response(
        address,
        "GET /v1/workspaces/workspace.desktoplab_fixture/files/preview?path=src/main.rs HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(preview.contains("200 OK"), "{preview}");
    assert!(preview.contains(r#""state":"text""#), "{preview}");
    assert!(preview.contains("[REDACTED_SECRET]"), "{preview}");
    assert!(!preview.contains("secret-value"), "{preview}");

    let protected_preview = read_response(
        address,
        "GET /v1/workspaces/workspace.desktoplab_fixture/files/preview?path=.env HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(protected_preview.contains("200 OK"), "{protected_preview}");
    assert!(
        protected_preview.contains(r#""state":"denied""#),
        "{protected_preview}"
    );
    assert!(
        protected_preview.contains(r#""deniedReason":"local_only_path""#),
        "{protected_preview}"
    );
    assert!(
        !protected_preview.contains("do-not-leak"),
        "{protected_preview}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn terminal_command_routes_require_approval_and_execute_when_approved() {
    let fixture = workspace_fixture();
    let workspace_root = fixture.path().join("terminal_fixture");
    fs::create_dir_all(&workspace_root).expect("workspace directory should exist");
    init_git_repo(&workspace_root);
    let server = test_router_server();
    let address = server.local_addr();
    let handle = server.spawn();

    complete_setup(address);
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&workspace_root),
        ),
        "200 OK",
        r#""workspaceId":"workspace.terminal_fixture""#,
    );

    let pending = read_response(
        address,
        &post_json_request(
            "/v1/workspaces/workspace.terminal_fixture/terminal/commands",
            r#"{"command":"printf pending","cwd":"","approvalRequired":true}"#,
        ),
    );
    assert!(pending.contains("200 OK"), "{pending}");
    assert!(
        pending.contains(r#""state":"approval_required""#),
        "{pending}"
    );
    assert!(pending.contains("printf pending"), "{pending}");
    assert!(!pending.contains(r#""stdout":"pending""#), "{pending}");

    let created_approval = read_response(
        address,
        &post_json_request(
            "/v1/approvals",
            r#"{"sessionId":"session.local","action":"terminal.command","operationId":"workspace.terminal_fixture:terminal.local","payload":{"command":"printf terminal-ok","cwd":""}}"#,
        ),
    );
    assert!(created_approval.contains("200 OK"), "{created_approval}");
    let approval_id = response_body_json(&created_approval)["approvalId"]
        .as_str()
        .expect("approval response should contain an id")
        .to_string();

    assert_response_contains(
        address,
        &post_json_request(
            &format!("/v1/approvals/{approval_id}/resolve"),
            r#"{"resolution":"approve"}"#,
        ),
        "200 OK",
        r#""state":"approved""#,
    );

    let approved = read_response(
        address,
        &post_json_request(
            "/v1/workspaces/workspace.terminal_fixture/terminal/commands",
            &format!(
                r#"{{"command":"printf terminal-ok","cwd":"","approvalRequired":true,"approvalId":"{approval_id}"}}"#
            ),
        ),
    );
    assert!(approved.contains("200 OK"), "{approved}");
    assert!(approved.contains(r#""state":"completed""#), "{approved}");
    assert!(approved.contains(r#""stdout":"terminal-ok""#), "{approved}");

    handle.shutdown().expect("server should stop");
}

#[test]
fn runtime_install_route_creates_executable_job_response() {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane.lock().unwrap().mark_ready();
    let server =
        ControlPlaneHttpServer::bind(HttpServerConfig::loopback(0).unwrap(), control_plane)
            .expect("server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    let response = read_response(
        address,
        &post_json_request("/v1/runtimes/runtime.ollama/install", "{}"),
    );
    let body = response_body_json(&response);

    assert!(response.contains("200 OK"), "{response}");
    assert_eq!(body["runtimeId"], "runtime.ollama", "{response}");
    assert!(
        matches!(
            body["state"].as_str(),
            Some("blocked" | "completed" | "failed")
        ),
        "{response}"
    );
    assert!(
        body["executionEvidence"]
            .as_str()
            .is_some_and(|evidence| evidence.contains("ollama --version")),
        "{response}"
    );
    assert!(
        body["verificationState"]
            .as_str()
            .is_some_and(|state| !state.is_empty() && state != "pending"),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn model_download_route_creates_executable_job_response() {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane.lock().unwrap().mark_ready();
    let server =
        ControlPlaneHttpServer::bind(HttpServerConfig::loopback(0).unwrap(), control_plane)
            .expect("server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    let response = read_response(
        address,
        &post_json_request(
            "/v1/models/model.qwen-coder-7b/download",
            r#"{"runtimeId":"runtime.ollama"}"#,
        ),
    );

    assert!(response.contains("200 OK"), "{response}");
    assert!(
        response.contains(r#""modelId":"model.qwen-coder-7b""#),
        "{response}"
    );
    assert!(response.contains(r#""state":"blocked""#), "{response}");
    assert!(
        response.contains(r#""blockedReason":"unknown model""#),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn known_model_download_route_waits_for_runtime_readiness() {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane.lock().unwrap().mark_ready();
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    let server = ControlPlaneHttpServer::bind_with_router(
        HttpServerConfig::loopback(0).unwrap(),
        control_plane,
        router,
    )
    .expect("server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    let response = read_response(
        address,
        &post_json_request(
            "/v1/models/model.gemma4-12b-q4/download",
            r#"{"runtimeId":"runtime.ollama"}"#,
        ),
    );

    assert!(response.contains("200 OK"), "{response}");
    assert!(
        response.contains(r#""modelId":"model.gemma4-12b-q4""#),
        "{response}"
    );
    assert!(
        response.contains(r#""runtimeId":"runtime.ollama""#),
        "{response}"
    );
    assert!(response.contains(r#""state":"blocked""#), "{response}");
    assert!(
        response.contains(r#""blockedReason":"runtime_not_verified""#),
        "{response}"
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn local_api_server_factory_starts_ready_loopback_router() {
    let server = desktoplab_control_plane::bind_unsafe_dev_local_api_server(0)
        .expect("dev local API server should bind");
    let address = server.local_addr();
    let handle = server.spawn();

    assert_response(
        address,
        "GET /v1/readiness HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#"{"state":"ready"}"#,
    );
    assert_response_contains(
        address,
        "GET /v1/setup/preview HTTP/1.1\r\nHost: localhost\r\n\r\n",
        "200 OK",
        r#""runtimeRecommendations""#,
    );

    handle.shutdown().expect("server should stop");
}

#[test]
fn control_plane_http_source_stays_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/http.rs",
        include_str!("../src/http.rs"),
        350,
    )
    .expect("control-plane http source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/http/agent_worker.rs",
        include_str!("../src/http/agent_worker.rs"),
        80,
    )
    .expect("agent HTTP worker should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/http/auth_response.rs",
        include_str!("../src/http/auth_response.rs"),
        60,
    )
    .expect("HTTP auth responses should stay below the line-count guard");
}

#[test]
fn local_api_router_source_stays_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router.rs",
        include_str!("../src/router.rs"),
        380,
    )
    .expect("local API router source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/test_fixtures.rs",
        include_str!("../src/router/test_fixtures.rs"),
        220,
    )
    .expect("router test fixtures should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes.rs",
        include_str!("../src/runtime_routes.rs"),
        120,
    )
    .expect("runtime routes source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes/helpers.rs",
        include_str!("../src/runtime_routes/helpers.rs"),
        160,
    )
    .expect("runtime route helpers source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes/helpers/body_fields.rs",
        include_str!("../src/runtime_routes/helpers/body_fields.rs"),
        80,
    )
    .expect("runtime route body field helpers should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes/helpers/host_target.rs",
        include_str!("../src/runtime_routes/helpers/host_target.rs"),
        80,
    )
    .expect("runtime route host target helpers should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes/setup_choice.rs",
        include_str!("../src/runtime_routes/setup_choice.rs"),
        80,
    )
    .expect("runtime setup choice source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/dispatch.rs",
        include_str!("../src/router/dispatch.rs"),
        360,
    )
    .expect("router dispatch source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/setup_runtime_model.rs",
        include_str!("../src/router/setup_runtime_model.rs"),
        360,
    )
    .expect("setup/runtime/model router source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/setup_runtime_model/ownership.rs",
        include_str!("../src/router/setup_runtime_model/ownership.rs"),
        80,
    )
    .expect("setup/runtime/model ownership helper should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/workspace_agent.rs",
        include_str!("../src/router/workspace_agent.rs"),
        360,
    )
    .expect("workspace/agent router source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/approvals.rs",
        include_str!("../src/router/approvals.rs"),
        160,
    )
    .expect("approval router source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/persistence.rs",
        include_str!("../src/router/persistence.rs"),
        160,
    )
    .expect("router persistence source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/persistence_save.rs",
        include_str!("../src/router/persistence_save.rs"),
        140,
    )
    .expect("router persistence save source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/persistence_agent.rs",
        include_str!("../src/router/persistence_agent.rs"),
        120,
    )
    .expect("router agent persistence source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/persistence_settings.rs",
        include_str!("../src/router/persistence_settings.rs"),
        80,
    )
    .expect("router persistence settings source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/helpers.rs",
        include_str!("../src/router/helpers.rs"),
        200,
    )
    .expect("router helper source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/helpers/query.rs",
        include_str!("../src/router/helpers/query.rs"),
        70,
    )
    .expect("router query helper source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/helpers/approval_json.rs",
        include_str!("../src/router/helpers/approval_json.rs"),
        80,
    )
    .expect("router approval helper source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/helpers/setup_selection.rs",
        include_str!("../src/router/helpers/setup_selection.rs"),
        80,
    )
    .expect("router setup selection helper should stay below the line-count guard");
}

fn assert_response(address: SocketAddr, request: &str, status: &str, body: &str) {
    let response = read_response(address, request);

    assert!(response.contains(status), "{response}");
    assert!(response.ends_with(body), "{response}");
}

fn assert_response_contains(address: SocketAddr, request: &str, status: &str, body_part: &str) {
    let response = read_response(address, request);

    assert!(response.contains(status), "{response}");
    assert!(response.contains(body_part), "{response}");
}

fn post_json_request(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn response_body_json(response: &str) -> Value {
    let (_, body) = response
        .split_once("\r\n\r\n")
        .expect("response should contain an HTTP body");
    serde_json::from_str(body).expect("response body should be JSON")
}

fn complete_setup(address: SocketAddr) {
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
        ),
        "200 OK",
        r#""state":"in_progress""#,
    );
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/runtimes/runtime.ollama/verify",
            r#"{"versionOutput":"ollama 0.5.0"}"#,
        ),
        "200 OK",
        r#""verificationState":"verified""#,
    );
    assert_response_contains(
        address,
        &post_json_request(
            "/v1/models/model.gemma4-12b-q4/verify",
            r#"{"inventoryOutput":"gemma4:12b    5.2 GB"}"#,
        ),
        "200 OK",
        r#""verificationState":"verified""#,
    );
    assert_response_contains(
        address,
        &post_json_request("/v1/setup/complete", "{}"),
        "200 OK",
        r#""state":"ready""#,
    );
}

fn test_router_server() -> ControlPlaneHttpServer {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane.lock().unwrap().mark_ready();
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b    5.2 GB"]);
    ControlPlaneHttpServer::bind_with_router(
        HttpServerConfig::loopback(0).unwrap(),
        control_plane,
        router,
    )
    .expect("server should bind")
}

fn workspace_fixture() -> TempDir {
    TempDir::new().expect("workspace fixture should be created")
}

fn init_git_repo(path: &Path) {
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
}

fn read_response(address: SocketAddr, request: &str) -> String {
    let mut last_response = String::new();
    for _ in 0..5 {
        let mut stream = TcpStream::connect(address).expect("server should accept connection");
        stream
            .write_all(request.as_bytes())
            .expect("request should write");
        let mut response = String::new();
        match stream.read_to_string(&mut response) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::ConnectionReset && !response.is_empty() => {}
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {
                last_response = response;
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(error) => panic!("response should read: {error}"),
        }
        if !response.is_empty() {
            return response;
        }
        last_response = response;
        thread::sleep(Duration::from_millis(10));
    }
    last_response
}
