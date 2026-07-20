use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn plugin_payload_separates_descriptor_provenance_and_execution_eligibility() {
    let mut router = LocalApiRouter::default();

    let payload = route_json(&mut router, "GET", "/v1/plugins", "");
    let plugin = &payload["plugins"][0];

    assert_eq!(plugin["descriptorState"], "present");
    assert_eq!(plugin["coldManifestState"], "present");
    assert_eq!(plugin["runtimeRegistration"], "not_registered");
    assert_eq!(plugin["installSource"], "bundled_descriptor");
    assert_eq!(plugin["integrityStatus"], "missing_signature");
    assert_eq!(plugin["executionEligibility"], "disabled");
    assert_eq!(
        plugin["provenance"]["blockedReasons"][0],
        "runtime_registration_missing"
    );
}

#[test]
fn unregistered_or_unpinned_plugins_are_blocked_by_default() {
    let mut router = LocalApiRouter::default();

    let payload = route_json(&mut router, "GET", "/v1/plugins", "");
    let plugin = &payload["plugins"][0];

    assert_eq!(plugin["status"], "blocked");
    assert!(
        plugin["blockedReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "plugin_integrity_missing_signature")
    );
    assert!(
        plugin["executionBoundary"]["reason"]
            .as_str()
            .unwrap()
            .contains("disabled")
    );
    assert!(!payload.to_string().contains("execution_ready"));
}

#[test]
fn plugin_routes_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/plugin_routes.rs",
        include_str!("../src/router/plugin_routes.rs"),
        180,
    )
    .expect("plugin routes should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
