use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn model_download_route_blocks_until_runtime_is_ready() {
    let mut router = LocalApiRouter::default();
    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    assert_eq!(download["source"], "service_backed");
    assert_eq!(download["state"], "blocked");
    assert_eq!(download["blockedReason"], "runtime_not_verified");
    assert!(download["jobId"].as_str().unwrap().starts_with("job."));
}

#[test]
fn model_download_ignores_client_supplied_runtime_ready() {
    let mut router = LocalApiRouter::default();
    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"runtimeReady":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    assert_eq!(download["source"], "service_backed");
    assert_eq!(download["state"], "blocked");
    assert_eq!(download["blockedReason"], "runtime_not_verified");
}

#[test]
fn model_download_starts_after_backend_runtime_verification_without_client_flag() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    assert_eq!(download["source"], "service_backed");
    assert_eq!(download["state"], "running");
    assert_eq!(download["runtimeId"], "runtime.ollama");
    assert_eq!(download["executionEvidence"], "ollama pull gemma4:12b");
}

#[test]
fn model_download_completion_marks_setup_ready_from_backend_execution() {
    let mut router = LocalApiRouter::default();
    router.complete_model_downloads_for_test();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.30.11");

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(download["state"], "completed");
    assert_eq!(
        state["readiness"]["evidence"]["modelVerification"]["state"],
        "verified"
    );
    assert_eq!(state["setup"]["state"], "ready");
    assert_eq!(state["setupPipeline"]["state"], "ready");
}

#[test]
fn model_download_reports_replace_choice_in_execution_contract() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.30.11");

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000,"setupChoice":"replace"}"#,
    );

    assert_eq!(download["state"], "running");
    assert_eq!(download["setupChoice"], "replace");
    assert_eq!(download["executionEvidence"], "ollama pull gemma4:12b");
}

#[test]
fn model_download_blocks_unknown_setup_choice_before_execution() {
    let mut router = LocalApiRouter::default();
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.30.11");

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000,"setupChoice":"maybe"}"#,
    );

    assert_eq!(download["state"], "blocked");
    assert_eq!(download["blockedReason"], "unknown setup choice");
}

#[test]
fn model_verify_persists_backend_model_readiness() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b", "llama3.1:8b"]);
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    let verified = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"client supplied inventory must be ignored"}"#,
    );

    assert_eq!(verified["verificationState"], "verified");
    assert_eq!(
        verified["readinessEvidence"]["modelVerification"]["state"],
        "verified"
    );
    let complete = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    assert_eq!(complete["setup"]["state"], "ready");
}

#[test]
fn model_verify_failure_persists_blocked_model_readiness() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["llama3.1:8b"]);
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        "{}",
    );
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b client spoof"}"#,
    );

    assert_eq!(blocked["verificationState"], "blocked");
    assert_eq!(blocked["blockedReason"], "model_not_reported_by_runtime");
    assert_eq!(
        blocked["readinessEvidence"]["modelVerification"]["state"],
        "blocked"
    );
}

#[test]
fn model_verify_rejects_client_supplied_inventory_output() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["llama3.1:8b"]);
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        "{}",
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b client spoof"}"#,
    );

    assert_eq!(blocked["verificationState"], "blocked");
    assert_eq!(blocked["blockedReason"], "model_not_reported_by_runtime");
}

#[test]
fn model_inventory_reflects_runtime_state() {
    let mut router = LocalApiRouter::default();
    let inventory = route_json(&mut router, "GET", "/v1/models", "");

    assert_eq!(inventory["source"], "service_backed");
    assert_eq!(inventory["models"][0]["installState"], "blocked");
    assert_eq!(
        inventory["models"][0]["blockedReason"],
        "runtime_not_verified"
    );
    assert_eq!(
        inventory["models"][0]["verification"],
        "runtime inventory required"
    );
}

#[test]
fn model_inventory_discovers_installed_models_and_keeps_candidates_hardware_gated() {
    let mut router = LocalApiRouter::default();
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.30.11");
    router.set_local_model_inventory_for_test(&[
        "NAME ID SIZE MODIFIED gemma4:12b dae161e27b0e 4.7 GB 50 seconds ago",
    ]);
    router.set_host_memory_gb_for_test(16);

    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    let gemma = inventory["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| model["modelId"] == "model.gemma4-12b-q4")
        .expect("Gemma should be listed");
    let larger = inventory["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| model["modelId"] == "model.gpt-oss-20b-mxfp4")
        .expect("larger candidate should be listed");

    assert_eq!(gemma["installState"], "installed");
    assert_eq!(gemma["compatibility"], "ready");
    assert_eq!(gemma["verification"], "Found in Ollama");
    assert_eq!(gemma["provenance"]["catalogSource"], "bundled_seed_catalog");
    assert_eq!(gemma["provenance"]["pullRef"], "gemma4:12b");
    assert_eq!(
        gemma["provenance"]["verificationState"],
        "verified_local_inventory"
    );
    assert_eq!(gemma["agentQualification"], "runtime_validation_required");
    assert_eq!(larger["installState"], "blocked");
    assert!(
        larger["blockedReason"]
            .as_str()
            .unwrap()
            .contains("Requires")
    );
}

#[test]
fn model_download_obeys_policy_for_unsafe_refs() {
    let mut router = LocalApiRouter::default();
    let unsafe_ref = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000,"pullRef":"../secret"}"#,
    );

    assert_eq!(unsafe_ref["state"], "blocked");
    assert_eq!(unsafe_ref["blockedReason"], "unsafe model reference");
}

#[test]
fn model_execution_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_model_execution.rs",
        include_str!("local_api_model_execution.rs"),
        270,
    )
    .expect("model execution route test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
