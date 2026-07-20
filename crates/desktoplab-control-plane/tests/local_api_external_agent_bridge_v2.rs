use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn external_agent_bridge_v2_contract_keeps_desktoplab_as_session_owner() {
    let mut router = LocalApiRouter::default();

    let contract = route_json(
        &mut router,
        "GET",
        "/v1/external-backends/bridge-contract/v2",
        "",
    );
    let inventory = route_json(&mut router, "GET", "/v1/external-backends", "");

    assert_eq!(contract["schemaVersion"], 2);
    assert_eq!(contract["status"], "contract_ready");
    assert_eq!(contract["sessionOwner"], "desktoplab");
    assert_eq!(
        contract["approvalBoundary"]["repositoryContextEgress"],
        "explicit_approval_required"
    );
    assert!(
        contract["normalizedEventKinds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|kind| kind == "agent.tool_request")
    );
    assert_eq!(
        inventory["bridgeContract"]["contractId"],
        contract["contractId"]
    );
}

#[test]
fn external_agent_bridge_v2_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_external_agent_bridge_v2.rs",
        include_str!("local_api_external_agent_bridge_v2.rs"),
        80,
    )
    .expect("external bridge v2 route test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
