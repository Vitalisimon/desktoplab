use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn prepared_workspace_routes_return_honest_blocked_state() {
    let mut router = LocalApiRouter::default();

    let refresh = router
        .route(
            "POST",
            "/v1/workspaces/workspace.desktoplab/intelligence/refresh",
            "{}",
        )
        .expect("refresh route should exist");
    let memory_delete = router
        .route("POST", "/v1/workspaces/memory/memory.1/delete", "{}")
        .expect("memory route should exist");
    let external_route = route_json(
        &mut router,
        "POST",
        "/v1/external-backends/routes/route.codex/resolve",
        r#"{"resolution":"approve"}"#,
    );

    assert_eq!(refresh.status(), "200 OK");
    let refresh: Value = serde_json::from_str(refresh.body()).expect("refresh should be json");
    assert_eq!(refresh["status"], "blocked");
    assert_eq!(refresh["reason"], "workspace_scan_refresh_not_available");
    assert_eq!(memory_delete.status(), "404 Not Found");
    assert_eq!(external_route["status"], "blocked");
    assert_eq!(external_route["reason"], "external_route_not_connected");
}

#[test]
fn job_retry_uses_real_job_state_instead_of_static_acceptance() {
    let mut router = LocalApiRouter::default();
    let job_id = router.create_retryable_job_for_test("model.download");

    let retry = route_json(
        &mut router,
        "POST",
        &format!("/v1/jobs/{}/retry", job_id),
        "{}",
    );
    let missing = router
        .route("POST", "/v1/jobs/job.missing/retry", "{}")
        .expect("retry route should exist");

    assert_eq!(retry["accepted"], true);
    assert_eq!(retry["state"], "queued");
    assert_eq!(missing.status(), "400 Bad Request");
    assert!(missing.body().contains("job_missing"));
}

#[test]
fn prepared_route_truth_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_prepared_route_truth.rs",
        include_str!("local_api_prepared_route_truth.rs"),
        140,
    )
    .expect("prepared route truth test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
