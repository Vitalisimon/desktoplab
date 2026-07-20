use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn model_download_rejects_unsafe_pull_references_before_execution() {
    for pull_ref in ["../secret", "qwen:7b; rm -rf /", "qwen:7b$(touch owned)"] {
        let mut router = LocalApiRouter::default();
        let blocked = route_json(
            &mut router,
            "POST",
            "/v1/models/model.gemma4-12b-q4/download",
            &format!(
                r#"{{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000,"pullRef":"{pull_ref}"}}"#
            ),
        );

        assert_eq!(blocked["state"], "blocked");
        assert_eq!(blocked["blockedReason"], "unsafe model reference");
        assert_eq!(blocked["jobId"], Value::Null);
    }
}

#[test]
fn model_download_uses_catalog_pull_ref_without_advanced_override() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    verify_runtime(&mut router);

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000,"setupChoice":"replace","pullRef":"llama3.1:8b"}"#,
    );

    assert!(
        matches!(download["state"].as_str(), Some("running" | "completed")),
        "{download}"
    );
    assert_eq!(download["executionEvidence"], "ollama pull gemma4:12b");
}

#[test]
fn unknown_model_id_cannot_be_used_as_an_arbitrary_pull_command() {
    let mut router = LocalApiRouter::default();
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/models/qwen:7b;rm-rf/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "unknown model");
    assert_eq!(blocked["jobId"], Value::Null);
}

#[test]
fn model_download_security_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/model_download_security.rs",
        include_str!("model_download_security.rs"),
        130,
    )
    .expect("model download security regression should stay focused");
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
