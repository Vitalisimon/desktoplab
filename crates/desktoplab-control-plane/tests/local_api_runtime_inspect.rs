use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn runtime_inspect_reports_active_route_and_live_evidence_separately() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let inspect = route_json(&mut router, "GET", "/v1/runtime/inspect", "");

    assert_eq!(inspect["source"], "service_backed");
    assert_eq!(
        inspect["active"]["selectedRouteId"],
        "route.local.gemma4-12b-q4"
    );
    assert_eq!(inspect["active"]["backendId"], "backend.ollama");
    assert_eq!(inspect["active"]["runtimeId"], "runtime.ollama");
    assert_eq!(inspect["active"]["modelId"], "model.gemma4-12b-q4");
    assert_eq!(inspect["active"]["accountMode"], "local_runtime");
    assert_eq!(inspect["active"]["egress"], "local_or_approval_gated");
    assert_eq!(
        inspect["active"]["toolCapability"],
        "filesystem_write_requires_approval"
    );
    assert_eq!(
        inspect["evidence"]["coldManifest"]["source"],
        "route_selection"
    );
    assert_eq!(inspect["evidence"]["liveRuntime"]["state"], "verified");
    assert_eq!(
        inspect["backendSupportContract"]["canonicalThreadOwner"],
        "desktoplab"
    );
}

#[test]
fn runtime_inspect_reports_degraded_reason_without_claiming_live_execution() {
    let mut router = LocalApiRouter::default();

    let inspect = route_json(&mut router, "GET", "/v1/runtime/inspect", "");

    assert_eq!(inspect["inspectState"], "blocked");
    assert_eq!(
        inspect["active"]["degradedReason"],
        "runtime_and_model_not_verified"
    );
    assert_eq!(inspect["evidence"]["liveRuntime"]["state"], "not_verified");
    assert_eq!(inspect["evidence"]["liveRuntime"]["evidence"], Value::Null);
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}
