use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn smollm3_inventory_exposes_the_live_evidence_scope() {
    let mut router = LocalApiRouter::default();
    let inventory = route_json(&mut router, "GET", "/v1/models", "");
    let smollm3 = inventory["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| model["modelId"] == "model.smollm3-3b-4bit-mlx")
        .unwrap();

    assert_eq!(smollm3["agentQualification"], "inspection_only");
    assert_eq!(smollm3["inspectionOnly"], true);
    assert_eq!(smollm3["mutationCapable"], false);
}

#[test]
fn inspection_model_contract_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_inspection_model.rs",
        include_str!("local_api_inspection_model.rs"),
        80,
    )
    .expect("inspection model contract test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_tool_scope.rs",
        include_str!("../src/router/agent_tool_scope.rs"),
        120,
    )
    .expect("inspection tool scope should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    serde_json::from_str(response.body()).unwrap()
}
