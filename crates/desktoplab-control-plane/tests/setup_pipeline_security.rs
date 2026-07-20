use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn setup_rejects_invalid_runtime_model_selection() {
    let mut router = LocalApiRouter::default();
    let rejected = router
        .route(
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.future","modelId":"model.gemma4-12b-q4"}"#,
        )
        .expect("route should exist");

    assert_eq!(rejected.status(), "400 Bad Request");
}

#[test]
fn setup_complete_rejects_client_readiness_override() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    let forced = route_json(
        &mut router,
        "POST",
        "/v1/setup/complete",
        r#"{"runtimeReady":true,"modelReady":true,"setupPipeline":{"state":"ready"}}"#,
    );

    assert_eq!(forced["setup"]["state"], "blocked");
    assert_eq!(forced["setupPipeline"]["state"], "blocked");
    assert_ne!(forced["readiness"]["state"], "ready");
}

#[test]
fn setup_rejects_unsafe_cached_installer_reference() {
    for cached_installer_path in [
        "../Ollama.dmg",
        "/Users/example/.ssh/id_rsa",
        "~/Desktop/Ollama.dmg",
    ] {
        let mut router = LocalApiRouter::default();
        let blocked = route_json(
            &mut router,
            "POST",
            "/v1/runtimes/runtime.ollama/install",
            &format!(
                r#"{{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64,"cachedInstallerPath":"{cached_installer_path}"}}"#
            ),
        );

        assert_eq!(blocked["state"], "blocked");
        assert_eq!(blocked["blockedReason"], "unsafe installer source");
        assert_eq!(blocked["retryClass"], "non_retryable");
        assert_eq!(blocked["jobId"], Value::Null);
    }
}

#[test]
fn setup_pipeline_security_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/setup_pipeline_security.rs",
        include_str!("setup_pipeline_security.rs"),
        130,
    )
    .expect("setup pipeline security regression should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
