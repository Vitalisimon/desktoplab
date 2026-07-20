use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn model_download_job_state_survives_router_restart() {
    let db = tempfile::NamedTempFile::new().expect("temp sqlite file");
    let mut router = LocalApiRouter::with_storage_path(db.path()).expect("router opens storage");
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

    assert_eq!(download["source"], "service_backed");
    assert_eq!(download["modelId"], "model.gemma4-12b-q4");
    let job_id = download["jobId"].as_str().expect("job id");
    assert!(job_id.starts_with("job."));

    let mut restarted = LocalApiRouter::with_storage_path(db.path()).expect("router restarts");
    let diagnostics = route_json(&mut restarted, "GET", "/v1/diagnostics", "");
    let summary = diagnostics["bundlePreview"]["summary"]
        .as_str()
        .expect("diagnostic summary");

    assert!(summary.contains("model.download:"), "{summary}");
    assert!(summary.contains("model.gemma4-12b-q4"), "{summary}");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
