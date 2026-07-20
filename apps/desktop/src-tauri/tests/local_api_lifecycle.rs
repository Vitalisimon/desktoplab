use desktoplab_control_plane::LocalApiRouter;
use desktoplab_desktop_lib::{
    run_user_terminal_command, LocalApiServer, PackagedLocalApi, PackagedLocalApiConfig,
};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use tempfile::TempDir;

#[test]
fn packaged_local_api_binds_random_loopback_port_and_exposes_bound_url() {
    let api = PackagedLocalApi::start(PackagedLocalApiConfig::random_loopback())
        .expect("packaged local api should start");

    assert_ne!(api.bound_addr().port(), 0);
    assert!(api.base_url().starts_with("http://127.0.0.1:"));
    assert!(!api.base_url().ends_with(":1421"));
}

#[test]
fn packaged_local_api_requires_native_token() {
    let api = PackagedLocalApi::start(PackagedLocalApiConfig::random_loopback())
        .expect("packaged local api should start");

    let unauthorized = read_response(
        api.bound_addr(),
        "GET /v1/version HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(unauthorized.contains("401 Unauthorized"), "{unauthorized}");

    let authorized = read_response(
        api.bound_addr(),
        &format!(
            "GET /v1/version HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\n\r\n",
            api.auth_token()
        ),
    );
    assert!(authorized.contains("200 OK"), "{authorized}");
}

#[test]
fn packaged_local_api_shutdown_stops_accepting_connections() {
    let api = PackagedLocalApi::start(PackagedLocalApiConfig::random_loopback())
        .expect("packaged local api should start");
    let address = api.bound_addr();

    api.shutdown().expect("packaged local api should stop");

    assert!(
        TcpStream::connect(address).is_err(),
        "shutdown should stop local api listener"
    );
}

#[test]
fn packaged_local_api_shutdown_runs_managed_runtime_cleanup() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    std::fs::create_dir_all(app_data.join("runtime")).expect("runtime dir should exist");
    std::fs::write(
        app_data.join("runtime/ollama-owned-by-desktoplab"),
        "desktop-session-1\n",
    )
    .expect("owned marker should write");
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_runtime_owner_id("desktop-session-1")
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    api.shutdown().expect("packaged local api should stop");

    let evidence =
        std::fs::read_to_string(&evidence_path).expect("runtime shutdown should be recorded");
    assert!(evidence.contains("ollama"), "{evidence}");
}

#[test]
fn packaged_local_api_shutdown_preserves_user_owned_ollama_without_owner_marker() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    api.shutdown().expect("packaged local api should stop");

    assert!(
        !evidence_path.exists(),
        "user-owned Ollama must not be quit without DesktopLab ownership marker"
    );
}

#[test]
fn packaged_local_api_shutdown_preserves_runtime_owned_by_another_desktop_session() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    let marker = app_data.join("runtime/ollama-owned-by-desktoplab");
    std::fs::create_dir_all(marker.parent().expect("marker parent should exist"))
        .expect("runtime dir should exist");
    std::fs::write(&marker, "desktop-session-old\n").expect("stale marker should write");
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_runtime_owner_id("desktop-session-current")
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    api.shutdown().expect("packaged local api should stop");

    assert!(
        !evidence_path.exists(),
        "stale ownership must not stop runtime"
    );
    assert_eq!(
        std::fs::read_to_string(marker).expect("stale marker should remain untouched"),
        "desktop-session-old\n"
    );
}

#[test]
fn packaged_local_api_shutdown_preserves_ready_user_owned_ollama_without_owner_marker() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    persist_ready_ollama_state(&app_data);
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    api.shutdown().expect("packaged local api should stop");

    assert!(
        !evidence_path.exists(),
        "persisted setup selection must not imply runtime ownership"
    );
}

#[test]
fn packaged_local_api_shutdown_preserves_user_owned_ollama_verified_after_start() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    authorized_request(
        &api,
        "POST /v1/setup/accept HTTP/1.1",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.qwen-coder-7b-q4"}"#,
    );
    authorized_request(
        &api,
        "POST /v1/runtimes/runtime.ollama/verify HTTP/1.1",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    authorized_request(
        &api,
        "POST /v1/models/model.qwen-coder-7b-q4/verify HTTP/1.1",
        r#"{"runtimeId":"runtime.ollama"}"#,
    );
    authorized_request(&api, "POST /v1/setup/complete HTTP/1.1", "{}");

    api.shutdown().expect("packaged local api should stop");

    assert!(
        !evidence_path.exists(),
        "verification of an existing runtime must not imply ownership"
    );
}

#[test]
fn local_api_server_window_close_shutdown_runs_managed_runtime_cleanup() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    std::fs::create_dir_all(app_data.join("runtime")).expect("runtime dir should exist");
    std::fs::write(
        app_data.join("runtime/ollama-owned-by-desktoplab"),
        "desktop-session-1\n",
    )
    .expect("owned marker should write");
    let evidence_path = fixture.path().join("window-close-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_runtime_owner_id("desktop-session-1")
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");
    let server = LocalApiServer::from_api_for_test(api);

    server.shutdown();

    let evidence =
        std::fs::read_to_string(&evidence_path).expect("window close shutdown should be recorded");
    assert!(evidence.contains("ollama"), "{evidence}");
}

#[test]
fn packaged_local_api_shutdown_removes_owner_marker_after_managed_cleanup() {
    let fixture = TempDir::new().expect("shutdown fixture should exist");
    let app_data = fixture.path().join("app-data");
    let marker = app_data.join("runtime/ollama-owned-by-desktoplab");
    std::fs::create_dir_all(marker.parent().expect("marker parent should exist"))
        .expect("runtime dir should exist");
    std::fs::write(&marker, "desktop-session-1\n").expect("owned marker should write");
    let evidence_path = fixture.path().join("managed-runtime-shutdown.txt");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(&app_data)
            .with_managed_runtime_owner_id("desktop-session-1")
            .with_managed_ollama_shutdown(true)
            .with_shutdown_evidence_path_for_test(&evidence_path),
    )
    .expect("packaged local api should start");

    api.shutdown().expect("packaged local api should stop");

    assert!(evidence_path.exists(), "managed runtime cleanup should run");
    assert!(
        !marker.exists(),
        "managed marker should be consumed after shutdown"
    );
}

fn persist_ready_ollama_state(app_data: &std::path::Path) {
    std::fs::create_dir_all(app_data).expect("app data dir should exist");
    let mut router = LocalApiRouter::with_storage_path(app_data.join("desktoplab.sqlite"))
        .expect("router should open");
    router.set_host_memory_gb_for_test(32);
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.qwen-coder-7b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test(
        "runtime.ollama",
        "model.qwen-coder-7b-q4",
        "qwen2.5-coder:7b",
    );
    let _ = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
}

fn route_json(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> serde_json::Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

#[test]
fn native_user_terminal_rejects_client_supplied_workspace_root() {
    let fixture = TempDir::new().expect("terminal fixture should exist");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should exist");

    let error = run_user_terminal_command(
        "workspace.native".to_string(),
        workspace.display().to_string(),
        "printf should-not-run".to_string(),
        Some(".".to_string()),
    )
    .expect_err("native terminal should not trust frontend supplied workspace roots");

    assert!(error.contains("local api terminal route"), "{error}");
}

#[test]
fn packaged_local_api_uses_app_data_state_and_writes_redacted_discovery() {
    let app_data = TempDir::new().expect("app data fixture should exist");
    let discovery_dir = TempDir::new().expect("discovery fixture should exist");
    let discovery_path = discovery_dir.path().join("local-api-discovery.json");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::random_loopback()
            .with_app_data_dir(app_data.path())
            .with_discovery_path(&discovery_path),
    )
    .expect("packaged local api should start with app data");

    assert!(app_data.path().join("desktoplab.sqlite").exists());
    let discovery = std::fs::read_to_string(&discovery_path).expect("discovery should persist");
    assert!(discovery.contains(api.base_url()), "{discovery}");
    assert!(discovery.contains(r#""tokenRedacted":"[REDACTED_LOCAL_API_TOKEN]""#));
    assert!(
        !discovery.contains(api.auth_token()),
        "discovery leaked native token"
    );

    api.shutdown().expect("packaged local api should stop");
    assert!(
        !discovery_path.exists(),
        "shutdown should invalidate packaged discovery"
    );
}

#[test]
fn packaged_user_home_config_sets_storage_and_discovery_paths() {
    let home = TempDir::new().expect("home fixture should exist");
    let discovery_path = home
        .path()
        .join(".config")
        .join("desktoplab")
        .join("local-api-discovery.json");
    let api = PackagedLocalApi::start(
        PackagedLocalApiConfig::for_user_home(home.path())
            .expect("packaged user home config should build"),
    )
    .expect("packaged local api should start with user home config");

    assert!(discovery_path.exists(), "discovery path should exist");
    assert!(home
        .path()
        .join(".config/desktoplab/desktoplab.sqlite")
        .exists());

    api.shutdown().expect("packaged local api should stop");
    assert!(!discovery_path.exists(), "shutdown removes discovery file");
}

fn read_response(address: std::net::SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("server should accept connection");
    stream
        .write_all(request.as_bytes())
        .expect("request should write");
    let mut response = String::new();
    match stream.read_to_string(&mut response) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::ConnectionReset && !response.is_empty() => {}
        Err(error) => panic!("response should read: {error}"),
    }
    response
}

fn authorized_request(api: &PackagedLocalApi, request_line: &str, body: &str) -> String {
    let request = format!(
        "{request_line}\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        api.auth_token(),
        body.len(),
        body
    );
    let response = read_response(api.bound_addr(), &request);
    assert!(response.contains("200 OK"), "{response}");
    response
}
