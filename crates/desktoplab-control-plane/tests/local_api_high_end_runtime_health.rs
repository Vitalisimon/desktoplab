use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use xtask::check_logical_line_limit;

#[test]
fn attached_user_runtime_is_probed_and_never_stopped_by_desktoplab() {
    let endpoint = serve_runtime(
        r#"{"data":[{"id":"model.large"}]}"#,
        r#"{"tokenizerReady":true,"gpuMemoryPressurePercent":72,"queueDepth":3}"#,
    );
    let mut router = LocalApiRouter::default();
    let attached = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/high-end/attach",
        &format!(
            r#"{{"runtimeId":"runtime.vllm","endpoint":"{endpoint}","modelId":"model.large"}}"#
        ),
    );

    assert_eq!(attached["state"], "model_ready", "{attached}");
    assert_eq!(attached["routeEligibility"], "verification_required");
    assert_eq!(attached["agentProtocolState"], "not_certified");
    assert_eq!(attached["ownership"], "user_owned");
    assert_eq!(attached["canStop"], false);
    assert_eq!(attached["evidence"]["endpointCompatible"], true);
    assert_eq!(attached["evidence"]["tokenizerReady"], true);
    assert_eq!(attached["evidence"]["gpuMemoryPressurePercent"], 72);
    assert_eq!(attached["evidence"]["queueDepth"], 3);

    let stop = router
        .route("POST", "/v1/runtimes/high-end/stop", "{}")
        .unwrap();
    assert_eq!(stop.status(), "400 Bad Request");
    assert_eq!(json(stop.body())["code"], "USER_OWNED_RUNTIME");
}

#[test]
fn unloaded_model_blocks_route_without_hiding_endpoint_health() {
    let endpoint = serve_runtime(
        r#"{"data":[{"id":"different.model"}]}"#,
        r#"{"tokenizerReady":true,"gpuMemoryPressurePercent":40,"queueDepth":0}"#,
    );
    let mut router = LocalApiRouter::default();
    let attached = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/high-end/attach",
        &format!(
            r#"{{"runtimeId":"runtime.nim","endpoint":"{endpoint}","modelId":"model.large"}}"#
        ),
    );

    assert_eq!(attached["state"], "model_loading", "{attached}");
    assert_eq!(attached["routeEligibility"], "blocked");
    assert_eq!(attached["evidence"]["endpointCompatible"], true);
    assert_eq!(attached["evidence"]["modelLoaded"], false);
}

#[test]
fn public_endpoint_is_rejected_before_network_access() {
    let mut router = LocalApiRouter::default();
    let response = router
        .route(
            "POST",
            "/v1/runtimes/high-end/attach",
            r#"{"runtimeId":"runtime.vllm","endpoint":"http://8.8.8.8:8000","modelId":"model.large"}"#,
        )
        .unwrap();

    assert_eq!(response.status(), "400 Bad Request");
    assert_eq!(
        json(response.body())["code"],
        "INVALID_LOCAL_RUNTIME_ENDPOINT"
    );
}

#[test]
fn high_end_runtime_route_test_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_high_end_runtime_health.rs",
        include_str!("local_api_high_end_runtime_health.rs"),
        220,
    )
    .expect("high-end runtime route tests should stay focused");
}

fn serve_runtime(models: &'static str, health: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for _ in 0..2 {
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
            let body = if String::from_utf8_lossy(&request).contains("/v1/models") {
                models
            } else {
                health
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            stream.flush().unwrap();
        }
    });
    format!("http://{address}")
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    json(response.body())
}

fn json(body: &str) -> Value {
    serde_json::from_str(body).unwrap()
}
