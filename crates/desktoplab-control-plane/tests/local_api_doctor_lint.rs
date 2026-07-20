use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn doctor_lint_exposes_stable_read_only_checks() {
    let mut router = LocalApiRouter::default();

    let lint = route_json(&mut router, "GET", "/v1/diagnostics/doctor/lint", "");

    assert_eq!(lint["source"], "service_backed");
    assert_eq!(lint["mode"], "lint");
    assert_eq!(lint["repairable"], false);
    assert_eq!(lint["summary"]["state"], "blocked");
    let checks = lint["checks"]
        .as_array()
        .expect("doctor lint checks should be array");
    let setup_check = checks
        .iter()
        .find(|check| check["checkId"] == "doctor.setup.runtime_model_ready")
        .expect("setup readiness check should be stable");
    assert_eq!(setup_check["severity"], "blocked");
    assert_eq!(setup_check["source"], "runtime");
    assert!(
        setup_check["message"]
            .as_str()
            .expect("message should be string")
            .contains("Setup"),
        "{setup_check}"
    );
    assert!(
        setup_check["fixHint"]
            .as_str()
            .expect("fix hint should be string")
            .contains("setup"),
        "{setup_check}"
    );
    assert!(
        !lint.to_string().contains("/Users/"),
        "doctor lint leaked protected paths: {lint}"
    );
    assert!(
        !lint.to_string().contains("token"),
        "doctor lint leaked token-like material: {lint}"
    );
    assert!(
        checks
            .iter()
            .any(|check| check["checkId"] == "doctor.storage.migrations_declared"),
        "doctor lint should include migration discipline check"
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
fn diagnostics_router_stays_below_doctor_lint_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/diagnostics.rs",
        include_str!("../src/router/diagnostics.rs"),
        380,
    )
    .expect("diagnostics router should stay below the doctor lint line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/diagnostics_doctor.rs",
        include_str!("../src/router/diagnostics_doctor.rs"),
        220,
    )
    .expect("doctor diagnostics should stay below the line-count guard");
}
