use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::thread;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn endpoint_health_alone_does_not_activate_a_high_end_agent_route() {
    let endpoint = serve_runtime_probes(4);
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("README.md"), "# High-end route\n").unwrap();
    assert!(
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&workspace)
            .output()
            .unwrap()
            .status
            .success()
    );

    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database)
        .expect("router should open storage");
    let attached = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/high-end/attach",
        &format!(
            r#"{{"runtimeId":"runtime.vllm","endpoint":"{endpoint}","modelId":"model.frontier"}}"#
        ),
    );
    assert_eq!(
        attached["routeEligibility"], "verification_required",
        "{attached}"
    );
    assert_eq!(
        route_json(&mut router, "GET", "/v1/app/state", "")["setup"]["state"],
        "not_started"
    );
    drop(router);

    let mut restored = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database)
        .expect("router should restore storage");
    let routes = route_json(&mut restored, "GET", "/v1/routing/options", "");
    assert_eq!(routes["selectedRouteId"], "route.local.unconfigured");
    assert!(!routes["options"].as_array().unwrap().iter().any(|option| {
        option["routeId"] == "route.high-end-local" && option["status"] == "available"
    }));
}

#[test]
fn high_end_route_persistence_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_high_end_route_persistence.rs",
        include_str!("local_api_high_end_route_persistence.rs"),
        170,
    )
    .expect("high-end route persistence test should remain focused");
}

fn serve_runtime_probes(request_count: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for _ in 0..request_count {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let mut chunk = [0_u8; 256];
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            let request = String::from_utf8_lossy(&request);
            let body = if request.contains("/v1/models") {
                r#"{"data":[{"id":"model.frontier"}]}"#
            } else {
                r#"{"tokenizerReady":true,"gpuMemoryPressurePercent":60,"queueDepth":0}"#
            };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            stream.flush().unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
        }
    });
    format!("http://{address}")
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
