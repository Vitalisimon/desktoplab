use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::NamedTempFile;

#[test]
fn plugin_trust_elevation_requires_approval_record() {
    let mut router = LocalApiRouter::default();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/plugins/plugin.acp/trust",
        r#"{"resolution":"approved"}"#,
    );

    assert_eq!(blocked["status"], "approval_required");
    assert_eq!(blocked["reason"], "approval_record_required");
}

#[test]
fn approved_trust_is_recorded_without_bypassing_execution_gates() {
    let mut router = LocalApiRouter::default();
    let requested = route_json(&mut router, "POST", "/v1/plugins/plugin.acp/trust", "{}");
    let approval_id = requested["approval"]["approvalId"].as_str().unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let recorded = route_json(
        &mut router,
        "POST",
        "/v1/plugins/plugin.acp/trust",
        &format!(r#"{{"approvalId":"{approval_id}"}}"#),
    );
    let listed = route_json(&mut router, "GET", "/v1/plugins", "");

    assert_eq!(recorded["status"], "recorded");
    assert_eq!(recorded["executionEligibility"], "disabled");
    assert_eq!(listed["plugins"][0]["trust"], "user_approved");
    assert_eq!(listed["plugins"][0]["status"], "blocked");
}

#[test]
fn unknown_plugin_cannot_acquire_a_trust_record() {
    let mut router = LocalApiRouter::default();
    let response = router
        .route("POST", "/v1/plugins/plugin.unknown/trust", "{}")
        .unwrap();

    assert_eq!(response.status(), "404 Not Found");
}

#[test]
fn approved_plugin_trust_survives_restart() {
    let database = NamedTempFile::new().unwrap();
    let mut router = LocalApiRouter::with_storage_path(database.path()).unwrap();
    let requested = route_json(&mut router, "POST", "/v1/plugins/plugin.acp/trust", "{}");
    let approval_id = requested["approval"]["approvalId"].as_str().unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/plugins/plugin.acp/trust",
        &format!(r#"{{"approvalId":"{approval_id}"}}"#),
    );
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(database.path()).unwrap();
    let listed = route_json(&mut restarted, "GET", "/v1/plugins", "");

    assert_eq!(listed["plugins"][0]["trust"], "user_approved");
    assert_eq!(listed["plugins"][0]["executionEligibility"], "disabled");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    serde_json::from_str(
        router
            .route(method, path, body)
            .expect("route should exist")
            .body(),
    )
    .expect("response should be json")
}
