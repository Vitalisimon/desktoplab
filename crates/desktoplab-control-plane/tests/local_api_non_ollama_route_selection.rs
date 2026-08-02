use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use xtask::check_logical_line_limit;

const OLLAMA_ROUTE: &str = "route.local.gemma4-12b-q4";
const LM_STUDIO_ROUTE: &str = "route.local.gpt-oss-20b-lm-studio";
const MLX_ROUTE: &str = "route.local.smollm3-3b-4bit-mlx";

#[test]
fn ready_lm_studio_route_certifies_native_tools_before_selection() {
    let mut router = ready_ollama_router();
    router.set_openai_compatible_canary_response_for_test(native_tool_response("."));

    let selected = select(&mut router, LM_STUDIO_ROUTE);

    assert_eq!(selected["selectedRouteId"], LM_STUDIO_ROUTE);
    assert_capability(
        &mut router,
        "backend.lm-studio",
        "desktoplab-gpt-oss-20b",
        "native_tools.v1",
    );
}

#[test]
fn ready_mlx_route_certifies_constrained_json_before_selection() {
    let mut router = ready_ollama_router();
    router.set_openai_compatible_canary_response_for_test(constrained_tool_response("."));

    let selected = select(&mut router, MLX_ROUTE);

    assert_eq!(selected["selectedRouteId"], MLX_ROUTE);
    assert_capability(
        &mut router,
        "backend.mlx-lm",
        "mlx-community/SmolLM3-3B-4bit",
        "constrained_json.v1",
    );
    let route = route_json(&mut router, "GET", "/v1/routing/preference", "");
    assert_eq!(route["backendDisplayName"], "MLX-LM");
    assert_eq!(route["backendKind"], "local");
    assert_eq!(route["modelDisplayName"], "SmolLM3 3B 4bit");
    assert_eq!(route["runtimeDisplayName"], "MLX-LM");
}

#[test]
fn lm_studio_selection_rolls_back_after_a_mismatched_native_tool_canary() {
    let mut router = ready_ollama_router();
    router.set_openai_compatible_canary_response_for_test(native_tool_response("/wrong"));

    assert_failed_selection_preserves_ollama(&mut router, LM_STUDIO_ROUTE);
}

#[test]
fn mlx_selection_rolls_back_after_malformed_constrained_json() {
    let mut router = ready_ollama_router();
    router.set_openai_compatible_canary_response_for_test(json!({
        "choices":[{"message":{"content":"I cannot call that tool."}}]
    }));

    assert_failed_selection_preserves_ollama(&mut router, MLX_ROUTE);
}

#[test]
fn non_ollama_route_selection_test_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_non_ollama_route_selection.rs",
        include_str!("local_api_non_ollama_route_selection.rs"),
        170,
    )
    .expect("non-Ollama route selection test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/local_model_protocol.rs",
        include_str!("../src/router/local_model_protocol.rs"),
        210,
    )
    .expect("non-Ollama model protocol certification should stay focused");
}

fn ready_ollama_router() -> LocalApiRouter {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);
    router.set_local_model_inventory_for_test(&[
        "gemma4:12b",
        "desktoplab-gpt-oss-20b",
        "mlx-community/SmolLM3-3B-4bit",
    ]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama ready");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma installed");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    router
}

fn select(router: &mut LocalApiRouter, route_id: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/routing/options/selection",
        &json!({"routeId":route_id}).to_string(),
    )
}

fn assert_capability(router: &mut LocalApiRouter, backend: &str, model: &str, protocol: &str) {
    let state = route_json(router, "GET", "/v1/app/state", "");
    let runtime = if backend == "backend.lm-studio" {
        "runtime.lm-studio"
    } else {
        "runtime.mlx-lm"
    };
    assert_eq!(state["readiness"]["evidence"]["runtimeId"], runtime);
    assert!(
        state["readiness"]["evidence"]["runtimeVerification"]["evidence"]
            .as_str()
            .is_some_and(|evidence| evidence.contains(runtime))
    );
    let capabilities = &state["readiness"]["evidence"]["modelCapabilities"];
    assert_eq!(capabilities["backendId"], backend);
    assert_eq!(capabilities["modelId"], model);
    assert_eq!(
        capabilities["toolProtocolCertification"]["protocol"],
        protocol
    );
}

fn assert_failed_selection_preserves_ollama(router: &mut LocalApiRouter, route_id: &str) {
    let response = router
        .route(
            "POST",
            "/v1/routing/options/selection",
            &json!({"routeId":route_id}).to_string(),
        )
        .expect("selection route should exist");
    let payload: Value = serde_json::from_str(response.body()).unwrap();
    assert_eq!(response.status(), "400 Bad Request");
    assert_eq!(payload["code"], "MODEL_AGENT_PROTOCOL_UNAVAILABLE");
    assert_eq!(
        route_json(router, "GET", "/v1/routing/options", "")["selectedRouteId"],
        OLLAMA_ROUTE
    );
}

fn native_tool_response(path: &str) -> Value {
    json!({"choices":[{"message":{"content":"","tool_calls":[{
        "id":"call-1","type":"function","function":{
            "name":"desktoplab.list_files","arguments":json!({"path":path}).to_string()
        }
    }]}}]})
}

fn constrained_tool_response(path: &str) -> Value {
    json!({"choices":[{"message":{"content":json!({
        "name":"desktoplab.list_files","arguments":{"path":path}
    }).to_string()}}]})
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
