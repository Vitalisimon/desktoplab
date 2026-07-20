use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn model_download_cancel_route_cancels_running_job_and_keeps_setup_incomplete() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    let job_id = download["jobId"].as_str().expect("job id");

    let cancel = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/cancel",
        &format!(r#"{{"jobId":"{job_id}"}}"#),
    );

    assert_eq!(cancel["source"], "service_backed");
    assert_eq!(cancel["state"], "cancelled");
    assert_eq!(cancel["jobId"], job_id);

    let complete = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    assert_ne!(complete["setup"]["state"], "ready");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
