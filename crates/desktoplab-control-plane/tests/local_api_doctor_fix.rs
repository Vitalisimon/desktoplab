use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn doctor_repairs_are_typed_and_safe_by_default() {
    let mut router = LocalApiRouter::default();

    let setup = route_json(
        &mut router,
        "POST",
        "/v1/diagnostics/repairs/repair.setup/run",
        "{}",
    );
    assert_eq!(setup["status"], "blocked");
    assert_eq!(setup["repairKind"], "guidance_only");
    assert_eq!(setup["requiresApproval"], false);
    assert_eq!(setup["sideEffects"].as_array().unwrap().len(), 0);
    assert_eq!(setup["reason"], "guidance_only_repair_requires_user_action");

    let jobs = route_json(
        &mut router,
        "POST",
        "/v1/diagnostics/repairs/repair.jobs/run",
        "{}",
    );
    assert_eq!(jobs["status"], "blocked");
    assert_eq!(jobs["repairKind"], "stale_state_cleanup");
    assert_eq!(jobs["reason"], "stale_state_cleanup_not_available");
}

#[test]
fn doctor_repairs_block_high_risk_or_unknown_actions() {
    let mut router = LocalApiRouter::default();

    let secret_rotation = route_json(
        &mut router,
        "POST",
        "/v1/diagnostics/repairs/repair.secret-rotation/run",
        r#"{"token":"sk-live-secret"}"#,
    );

    assert_eq!(secret_rotation["status"], "blocked");
    assert_eq!(secret_rotation["repairKind"], "unsupported");
    assert_eq!(secret_rotation["reason"], "unsupported_diagnostic_repair");
    assert!(
        !secret_rotation.to_string().contains("sk-live-secret"),
        "repair response leaked request secret: {secret_rotation}"
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
