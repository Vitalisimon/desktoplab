use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn setup_preview_separates_certified_preview_and_planned_runtimes() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");
    let runtimes = preview["runtimeRecommendations"]
        .as_array()
        .expect("runtime recommendations");

    assert_capability(runtimes, "runtime.ollama", "certified", "managed");
    assert_capability(runtimes, "runtime.lm-studio", "planned", "external_only");
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_capability(runtimes, "runtime.mlx-lm", "experimental", "none");
    }
}

#[test]
fn runtime_inventory_disables_uncertified_install_routes() {
    let mut router = LocalApiRouter::default();
    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");
    let runtimes = inventory["runtimes"].as_array().expect("runtime inventory");

    let ollama = runtime(runtimes, "runtime.ollama");
    assert_eq!(ollama["runtimeCapability"]["availability"], "certified");
    assert_eq!(ollama["install"]["supported"], true);

    for runtime_id in ["runtime.lm-studio", "runtime.mlx-lm"] {
        let item = runtime(runtimes, runtime_id);
        assert_eq!(item["install"]["supported"], false);
    }
}

#[test]
fn direct_uncertified_runtime_install_fails_closed_before_execution() {
    let mut router = LocalApiRouter::default();
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.mlx-lm/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "runtime setup is not available");
    assert_eq!(blocked["retryClass"], "non_retryable");
}

#[test]
fn local_runtime_capability_contract_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_runtime_capability_contract.rs",
        include_str!("local_runtime_capability_contract.rs"),
        100,
    )
    .expect("runtime capability contract test should stay focused");
}

fn assert_capability(runtimes: &[Value], runtime_id: &str, availability: &str, mode: &str) {
    let capability = &runtime(runtimes, runtime_id)["runtimeCapability"];
    assert_eq!(capability["availability"], availability);
    assert_eq!(capability["setupMode"], mode);
    assert!(capability["certifiedPlatforms"].is_array());
    assert!(capability["evidenceScope"].is_string());
}

fn runtime<'a>(runtimes: &'a [Value], runtime_id: &str) -> &'a Value {
    runtimes
        .iter()
        .find(|runtime| runtime["manifestId"] == runtime_id || runtime["runtimeId"] == runtime_id)
        .unwrap_or_else(|| panic!("{runtime_id} should be present"))
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
