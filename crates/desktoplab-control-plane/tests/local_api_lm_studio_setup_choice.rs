use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn use_existing_lm_studio_verifies_without_managed_install_terms() {
    let mut router = LocalApiRouter::default();
    let response = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.lm-studio/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64,"setupChoice":"use_existing"}"#,
    );

    assert_eq!(response["setupChoice"], "use_existing");
    assert!(matches!(
        response["state"].as_str(),
        Some("completed" | "blocked")
    ));
    assert!(
        !response["remediation"]
            .as_str()
            .is_some_and(|copy| copy.contains("vendor terms"))
    );
}

#[test]
fn lm_studio_setup_choice_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_lm_studio_setup_choice.rs",
        include_str!("local_api_lm_studio_setup_choice.rs"),
        80,
    )
    .expect("LM Studio setup-choice test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
