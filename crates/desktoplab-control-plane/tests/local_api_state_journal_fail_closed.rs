use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn journal_fault_blocks_mutations_but_keeps_diagnostics_available() {
    let mut router = LocalApiRouter::default();
    router.inject_state_journal_fault_for_test("fixture_disk_write_failed");

    let blocked = route(&mut router, "POST", "/v1/approvals", r#"{"action":"test"}"#);
    assert_eq!(blocked.status(), "500 Internal Server Error");
    assert_eq!(json(&blocked)["code"], "STATE_JOURNAL_FAILED");

    let diagnostics = route(&mut router, "GET", "/v1/diagnostics", "");
    assert_eq!(diagnostics.status(), "200 OK");
    let diagnostics = json(&diagnostics);
    assert_eq!(diagnostics["state"], "degraded");
    assert!(
        diagnostics["services"]
            .as_array()
            .unwrap()
            .iter()
            .any(|service| {
                service["family"] == "state_journal" && service["state"] == "failed"
            })
    );

    router.clear_state_journal_fault_for_test();
    let approvals = json(&route(&mut router, "GET", "/v1/approvals", ""));
    assert!(approvals["approvals"].as_array().unwrap().is_empty());
}

fn route(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> desktoplab_control_plane::ApiRouteResponse {
    router
        .route(method, path, body)
        .expect("route should exist")
}

fn json(response: &desktoplab_control_plane::ApiRouteResponse) -> Value {
    serde_json::from_str(response.body()).expect("response should be JSON")
}
