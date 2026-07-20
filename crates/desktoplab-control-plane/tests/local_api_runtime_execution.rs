use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn runtime_install_route_runs_executor_or_blocks_with_evidence() {
    let mut router = LocalApiRouter::default();
    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    assert_eq!(install["source"], "service_backed");
    assert!(install["jobId"].as_str().unwrap().starts_with("job."));
    assert_ne!(install["state"], "queued");
    assert!(
        install["executionEvidence"]
            .as_str()
            .unwrap()
            .contains("ollama")
    );
}

#[test]
fn runtime_inventory_reflects_host_detection() {
    let mut router = LocalApiRouter::default();
    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");

    assert_eq!(inventory["source"], "service_backed");
    assert_eq!(inventory["runtimes"][0]["detectionSource"], "host_probe");
    assert!(
        matches!(
            inventory["runtimes"][0]["status"].as_str(),
            Some("ready" | "installed" | "not_installed" | "degraded" | "unknown")
        ),
        "{inventory}"
    );
}

#[test]
fn runtime_verification_updates_setup_state() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let verified = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );

    assert_eq!(verified["source"], "service_backed");
    assert_eq!(verified["runtimeId"], "runtime.ollama");
    assert_eq!(verified["verificationState"], "verified");
}

#[test]
fn runtime_verify_rejects_client_supplied_version_output() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(false, "backend detected missing runtime");

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 999.0 client spoof"}"#,
    );

    assert_eq!(blocked["verificationState"], "blocked");
    assert_eq!(blocked["blockedReason"], "runtime_not_detected");
    assert_eq!(
        blocked["readinessEvidence"]["runtimeVerification"]["state"],
        "blocked"
    );
}

#[test]
fn runtime_install_result_updates_readiness_from_backend_execution() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    let expected = if install["state"] == "completed" {
        "verified"
    } else {
        "blocked"
    };
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["state"],
        expected
    );
}

#[test]
fn runtime_install_block_updates_setup_pipeline_instead_of_leaving_running_placeholder() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64}"#,
    );
    let state = route_json(&mut router, "GET", "/v1/app/state", "");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(state["setupPipeline"]["state"], "blocked");
    assert_eq!(
        state["setupPipeline"]["blockedReason"],
        "network unavailable"
    );
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["state"],
        "blocked"
    );
}

#[test]
fn runtime_install_blocks_unknown_setup_choice_before_execution() {
    let mut router = LocalApiRouter::default();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64,"setupChoice":"maybe"}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "unknown setup choice");
}

#[test]
fn lm_studio_install_is_guided_and_does_not_mark_runtime_ready() {
    let mut router = LocalApiRouter::default();
    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.lm-studio/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    assert_eq!(install["state"], "external_guided");
    assert_eq!(install["verificationState"], "requires_external_app");
    assert!(
        install["remediation"]
            .as_str()
            .unwrap()
            .contains("open LM Studio manually")
    );

    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["state"],
        "blocked"
    );
}

#[test]
fn runtime_verify_persists_backend_runtime_readiness() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let verified = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );

    assert_eq!(
        verified["readinessEvidence"]["runtimeVerification"]["state"],
        "verified"
    );
    let state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(
        state["readiness"]["evidence"]["runtimeVerification"]["evidence"],
        "backend detected ollama 0.5.0"
    );
    assert_eq!(
        state["readiness"]["evidence"]["blockedReason"],
        "model_not_verified"
    );
}

#[test]
fn runtime_verify_failure_persists_blocked_runtime_readiness() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(false, "backend detected missing runtime");
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 999.0 client spoof"}"#,
    );

    assert_eq!(
        blocked["readinessEvidence"]["runtimeVerification"]["state"],
        "blocked"
    );
    assert_eq!(
        blocked["readinessEvidence"]["blockedReason"],
        "runtime_and_model_not_verified"
    );
}

#[test]
fn runtime_execution_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_runtime_execution.rs",
        include_str!("local_api_runtime_execution.rs"),
        240,
    )
    .expect("runtime execution route test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
