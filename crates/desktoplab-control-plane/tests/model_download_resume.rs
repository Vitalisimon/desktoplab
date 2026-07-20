use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn model_download_resume_after_cancel_starts_new_runtime_pull_job() {
    let mut router = ready_router();
    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    let job_id = download["jobId"].as_str().expect("job id");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/cancel",
        &format!(r#"{{"jobId":"{job_id}"}}"#),
    );

    let resumed = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/resume",
        &format!(r#"{{"jobId":"{job_id}","networkAvailable":true,"diskAvailableMb":100000}}"#),
    );

    assert_eq!(resumed["state"], "running");
    assert_eq!(resumed["modelId"], "model.gemma4-12b-q4");
    assert_eq!(resumed["runtimeId"], "runtime.ollama");
    assert_ne!(resumed["jobId"], job_id);
    assert_eq!(resumed["previousJobId"], job_id);
    assert_eq!(resumed["resume"], true);
    assert_eq!(resumed["resumeMode"], "runtime_pull_resume");
    assert_eq!(resumed["retryClass"], "retryable");
}

#[test]
fn model_download_resume_reports_unsupported_resume_honestly() {
    let mut router = ready_router();
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/resume",
        r#"{"jobId":"job.missing","resumeSupported":false}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "resume unsupported");
    assert_eq!(blocked["retryClass"], "non_retryable");
}

#[test]
fn model_download_resume_keeps_network_failure_retryable() {
    let mut router = ready_router();
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/resume",
        r#"{"jobId":"job.missing","networkAvailable":false,"diskAvailableMb":100000}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "network unavailable");
    assert_eq!(blocked["retryClass"], "offline");
}

fn ready_router() -> LocalApiRouter {
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
    router
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
