use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn critical_routes_do_not_return_success_like_fallbacks() {
    let mut router = LocalApiRouter::default();

    let app_state = route_json(&mut router, "GET", "/v1/app/state", "");
    assert_eq!(app_state["setup"]["state"], "not_started");
    assert_ne!(app_state["readiness"]["state"], "ready");

    let agent = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert!(agent["session"].is_null());

    let diagnostics = route_json(&mut router, "GET", "/v1/diagnostics", "");
    assert_ne!(diagnostics["state"], "ready");
}

#[test]
fn setup_accept_without_selection_is_blocked_not_defaulted() {
    let mut router = LocalApiRouter::default();

    let response = router
        .route("POST", "/v1/setup/accept", "{}")
        .expect("route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(!response.body().contains("runtime.ollama"));
}

#[test]
fn dev_test_reset_is_hidden_by_default() {
    let mut router = LocalApiRouter::default();

    let response = router
        .route("POST", "/v1/test/reset", "{}")
        .expect("v1 route should be handled");

    assert_eq!(response.status(), "404 Not Found");
}

#[test]
fn fallback_guard_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_no_product_fallbacks.rs",
        include_str!("local_api_no_product_fallbacks.rs"),
        180,
    )
    .expect("fallback guard should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
