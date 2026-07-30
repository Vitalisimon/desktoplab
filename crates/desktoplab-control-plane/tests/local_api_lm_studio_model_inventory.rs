use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn compatible_existing_lm_studio_model_is_ready_on_supported_hosts() {
    let mut router = LocalApiRouter::default();
    router.mark_runtime_verified_for_test("runtime.lm-studio", "existing LM Studio ready");
    router.set_local_model_inventory_for_test(&["desktoplab-gpt-oss-20b"]);

    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    let model = inventory["models"]
        .as_array()
        .expect("models")
        .iter()
        .find(|model| model["modelId"] == "model.gpt-oss-20b-lm-studio")
        .expect("LM Studio model");

    if supports_lm_studio() {
        assert_eq!(model["installState"], "installed");
        assert_eq!(model["compatibility"], "ready");
        assert_eq!(model["verification"], "Found in LM Studio");
    } else {
        assert_eq!(model["installState"], "blocked");
    }
}

#[test]
fn lm_studio_model_inventory_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_lm_studio_model_inventory.rs",
        include_str!("local_api_lm_studio_model_inventory.rs"),
        80,
    )
    .expect("LM Studio model inventory test should stay focused");
}

fn supports_lm_studio() -> bool {
    matches!(
        (std::env::consts::OS, std::env::consts::ARCH),
        ("macos", "aarch64") | ("linux", "x86_64" | "aarch64")
    )
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
