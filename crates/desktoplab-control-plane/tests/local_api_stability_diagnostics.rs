use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn diagnostics_expose_bounded_payload_free_stability_snapshot() {
    let mut router = LocalApiRouter::default();
    let snapshot = route_json(&mut router, "GET", "/v1/diagnostics", "");
    let stability = &snapshot["stability"];

    assert_eq!(stability["kind"], "desktoplab.stability.snapshot");
    assert_eq!(stability["schemaVersion"], 1);
    assert_eq!(stability["redacted"], true);
    assert_eq!(stability["payloadFree"], true);
    assert_eq!(stability["startupPhase"], "setup_pending");
    assert_eq!(stability["localApiHealth"]["state"], "responding");
    assert_eq!(stability["routeDecisionRecency"]["state"], "current");
    assert_eq!(
        stability["routeDecisionRecency"]["selectedRouteId"],
        "route.local.unconfigured"
    );
    assert!(stability["uptimeMs"].as_u64().is_some(), "{stability}");
    assert_eq!(stability["queueBackpressure"]["state"], "idle");
    assert_eq!(stability["queueBackpressure"]["payloadFree"], true);
    assert_eq!(stability["budgets"]["memory"]["sampleState"], "not_sampled");
    assert_eq!(stability["budgets"]["disk"]["sampleState"], "not_sampled");
    assert!(
        stability["degradedReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "setup_not_ready"),
        "{stability}"
    );
}

#[test]
fn stability_snapshot_reflects_setup_queue_without_leaking_payloads() {
    let mut router = LocalApiRouter::default();
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4","prompt":"sk-live-secret"}"#,
    );

    let snapshot = route_json(&mut router, "GET", "/v1/diagnostics", "");
    let stability = &snapshot["stability"];

    assert_eq!(
        stability["queueBackpressure"]["state"],
        "attention_required"
    );
    assert_eq!(stability["queueBackpressure"]["blocked"], 1);
    assert!(
        stability["jobStates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| job["kind"] == "model.download" && job["state"] == "blocked"),
        "{stability}"
    );
    assert!(
        !stability.to_string().contains("sk-live-secret"),
        "stability leaked secret-like input: {stability}"
    );
    assert!(
        !stability.to_string().contains("/Users/"),
        "stability leaked private path: {stability}"
    );
    assert!(
        !stability.to_string().contains("prompt"),
        "stability leaked prompt payload: {stability}"
    );
}

#[test]
fn stability_diagnostics_router_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/stability_diagnostics.rs",
        include_str!("../src/router/stability_diagnostics.rs"),
        180,
    )
    .expect("stability diagnostics router should stay below its line-count guard");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn check_logical_line_limit(path: &str, source: &str, limit: usize) -> Result<(), String> {
    let logical_lines = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count();
    if logical_lines > limit {
        return Err(format!(
            "{path} has {logical_lines} logical lines; limit is {limit}. Extract responsibilities before adding more."
        ));
    }
    Ok(())
}
