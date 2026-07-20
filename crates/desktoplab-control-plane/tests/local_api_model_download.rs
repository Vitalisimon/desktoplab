use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn model_routes_reflect_service_inventory_and_download_planner() {
    let mut router = LocalApiRouter::default();
    router.complete_model_downloads_for_test();

    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    assert_eq!(inventory["source"], "service_backed");
    assert_eq!(
        inventory["models"][0]["modelId"],
        "model.nemotron-3-nano-4b-q4"
    );
    assert_eq!(inventory["models"][0]["runtimeId"], "runtime.ollama");
    assert_eq!(inventory["models"][0]["compatibility"], "compatible");

    verify_runtime(&mut router);
    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    assert_eq!(download["source"], "service_backed");
    assert_eq!(download["modelId"], "model.gemma4-12b-q4");
    assert_eq!(download["runtimeId"], "runtime.ollama");
    assert!(
        matches!(download["state"].as_str(), Some("running" | "completed")),
        "{download}"
    );
    assert!(download["jobId"].as_str().unwrap().starts_with("job."));
}

#[test]
fn model_inventory_exposes_curated_agent_candidates_and_excludes_unsuitable_models() {
    let mut router = LocalApiRouter::default();

    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    let ids = inventory["models"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|model| model["modelId"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 19);
    assert!(!ids.contains(&"model.qwen-coder-7b-q4"));
    assert!(!ids.contains(&"model.qwen-coder-14b-q4"));
    assert!(ids.contains(&"model.gemma4-12b-q4"));
    assert!(ids.contains(&"model.qwen3-coder-next-80b-q4"));
    assert!(ids.contains(&"model.qwen3-coder-480b-q4"));
    assert!(inventory["models"].as_array().unwrap().iter().all(|model| {
        model["recommended"] == false
            && model["agentQualification"] == "runtime_validation_required"
    }));

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.glm-5.2-cloud/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    assert_blocked(&download, "non_retryable", "unknown model");
}

#[test]
fn modern_agent_candidate_download_uses_its_official_ollama_tag() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    router.set_local_model_inventory_for_test(&[]);
    verify_runtime(&mut router);

    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    let gemma = inventory["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| model["modelId"] == "model.gemma4-12b-q4")
        .unwrap();
    assert_eq!(gemma["installState"], "downloadable");
    assert_eq!(gemma["agentQualification"], "runtime_validation_required");

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    assert_eq!(download["state"], "running");
    assert_eq!(download["executionEvidence"], "ollama pull gemma4:12b");
}

#[test]
fn model_download_returns_distinct_blocked_states() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(512);

    let offline = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableMb":100000}"#,
    );
    assert_blocked(&offline, "offline", "network unavailable");

    let disk = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":128}"#,
    );
    assert_blocked(&disk, "user_action", "insufficient disk");
    assert_eq!(disk["requiredDiskMb"], 7600);
    assert_eq!(disk["availableDiskMb"], 128);

    let workstation_disk = route_json(
        &mut router,
        "POST",
        "/v1/models/model.qwen3-coder-480b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":1000}"#,
    );
    assert_blocked(&workstation_disk, "user_action", "insufficient disk");
}

#[test]
fn model_download_blocks_larger_candidate_when_memory_is_insufficient() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(16);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gpt-oss-20b-mxfp4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    assert_blocked(&blocked, "user_action", "not enough memory for this model");
    assert_eq!(blocked["requiredMemoryGb"], 24);
    assert_eq!(blocked["availableMemoryGb"], 16);
}

#[test]
fn model_download_resume_starts_new_runtime_pull_job_for_existing_job() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    verify_runtime(&mut router);
    let first = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    let job_id = first["jobId"].as_str().unwrap();

    let resume = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download/resume",
        &format!(r#"{{"jobId":"{job_id}"}}"#),
    );

    assert_eq!(resume["source"], "service_backed");
    assert!(resume["jobId"].as_str().unwrap().starts_with("job."));
    assert_ne!(resume["jobId"], job_id);
    assert_eq!(resume["previousJobId"], job_id);
    assert_eq!(resume["resume"], true);
    assert_eq!(resume["resumeMode"], "runtime_pull_resume");
    assert_eq!(resume["state"], "running");
}

#[test]
fn local_api_model_download_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_model_download.rs",
        include_str!("local_api_model_download.rs"),
        190,
    )
    .expect("model download route test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/model_routes.rs",
        include_str!("../src/model_routes.rs"),
        280,
    )
    .expect("model route source should stay focused");
}

fn assert_blocked(payload: &Value, retry_class: &str, reason: &str) {
    assert_eq!(payload["source"], "service_backed");
    assert_eq!(payload["state"], "blocked");
    assert_eq!(payload["retryClass"], retry_class);
    assert_eq!(payload["blockedReason"], reason);
}

fn verify_runtime(router: &mut LocalApiRouter) {
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.30.11");
    let _ = route_json(
        router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
