use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn security_audit_is_separate_redacted_and_stable() {
    let mut router = LocalApiRouter::default();

    let audit = route_json(&mut router, "GET", "/v1/security/audit", "");

    assert_eq!(audit["source"], "service_backed");
    assert_eq!(audit["kind"], "security_audit");
    assert_eq!(audit["redacted"], true);
    assert_eq!(audit["exportSafe"], true);
    assert_eq!(
        audit["remediationPolicy"],
        "safe_remediation_routes_through_doctor_repair_contract"
    );
    let findings = audit["findings"].as_array().unwrap();
    for check_id in [
        "security.local_only.posture",
        "security.provider_egress.approval_gated",
        "security.approval_mode.current",
        "security.workspace.protected_paths",
        "security.plugins.provenance",
        "security.backends.trust_level",
        "security.redaction.export_ready",
    ] {
        assert!(
            findings
                .iter()
                .any(|finding| finding["checkId"] == check_id),
            "{check_id} missing from security audit"
        );
    }
    assert!(
        !audit.to_string().contains("/Users/"),
        "security audit leaked private absolute path: {audit}"
    );
    assert!(
        !audit.to_string().contains("sk-live"),
        "security audit leaked token-like material: {audit}"
    );
}

#[test]
fn plugin_runtime_is_blocked_not_claimed_executable() {
    let mut router = LocalApiRouter::default();

    let audit = route_json(&mut router, "GET", "/v1/security/audit", "");
    let plugin = audit["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["checkId"] == "security.plugins.provenance")
        .expect("plugin provenance finding should exist");

    assert_eq!(plugin["severity"], "blocked");
    assert_eq!(plugin["suppressed"], true);
    assert_ne!(audit["summary"]["state"], "blocked");
    assert!(
        plugin["message"]
            .as_str()
            .unwrap()
            .contains("not certified")
    );
}

#[test]
fn security_audit_router_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/security_audit.rs",
        include_str!("../src/router/security_audit.rs"),
        220,
    )
    .expect("security audit router should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
