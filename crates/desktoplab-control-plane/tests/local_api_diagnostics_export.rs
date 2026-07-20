use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn diagnostics_export_is_bounded_and_redacted() {
    let mut router = LocalApiRouter::default();

    let export = route_json(&mut router, "GET", "/v1/diagnostics/export", "");

    assert_eq!(export["manifest"]["kind"], "desktoplab.diagnostics.export");
    assert_eq!(export["manifest"]["schemaVersion"], 1);
    assert_eq!(export["summary"]["redacted"], true);
    assert_eq!(export["redaction"]["promptsIncluded"], false);
    assert_eq!(export["redaction"]["rawToolOutputIncluded"], false);
    assert_eq!(export["redaction"]["secretsIncluded"], false);
    assert_eq!(export["redaction"]["privatePathsIncluded"], false);
    assert_eq!(export["reviewBeforeSharing"], true);
    assert!(
        export["serviceStates"].as_array().unwrap().len() >= 3,
        "{export}"
    );
    assert!(
        export["routeFacts"]["selectedRouteId"]
            .as_str()
            .unwrap()
            .starts_with("route."),
        "{export}"
    );
    assert!(
        export["summary"]["sizeBytes"].as_u64().unwrap()
            <= export["summary"]["maxBytes"].as_u64().unwrap(),
        "{export}"
    );
    assert!(
        !export.to_string().contains("/Users/"),
        "diagnostics export leaked private path: {export}"
    );
    assert!(
        !export.to_string().contains("sk-live-secret"),
        "diagnostics export leaked secret: {export}"
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

#[test]
fn diagnostics_export_router_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/diagnostics_export.rs",
        include_str!("../src/router/diagnostics_export.rs"),
        220,
    )
    .expect("diagnostics export router should stay below its line-count guard");
}
