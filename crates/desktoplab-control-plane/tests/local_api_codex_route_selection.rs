use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::net::TcpListener;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn codex_route_becomes_selectable_only_after_bridge_pairing() {
    let (temp, listener, mut router) = codex_router_fixture();
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );

    let before = route_json(&mut router, "GET", "/v1/routing/options", "");
    assert_eq!(before["options"][2]["routeId"], "route.external.codex");
    assert_eq!(before["options"][2]["status"], "unavailable");

    complete_codex_bridge(
        &mut router,
        &format!("http://{}", listener.local_addr().unwrap()),
    );

    let after = route_json(&mut router, "GET", "/v1/routing/options", "");
    assert_eq!(after["options"][2]["routeId"], "route.external.codex");
    assert_eq!(after["options"][2]["status"], "available");
    assert_eq!(after["options"][2]["egressPolicy"], "requires_approval");
    assert_eq!(
        after["options"][2]["repositoryContextEgress"],
        "approval_required"
    );
    assert_eq!(after["options"][2]["disabledReason"], Value::Null);

    let selected = route_json(
        &mut router,
        "POST",
        "/v1/routing/options/selection",
        r#"{"routeId":"route.external.codex"}"#,
    );

    assert_eq!(selected["selectedRouteId"], "route.external.codex");
    drop((temp, listener));
}

#[test]
fn codex_route_selection_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_codex_route_selection.rs",
        include_str!("local_api_codex_route_selection.rs"),
        110,
    )
    .expect("codex route selection test should stay focused");
}

fn codex_router_fixture() -> (TempDir, TcpListener, LocalApiRouter) {
    let temp = tempfile::tempdir().expect("bridge dir should be created");
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback responder should bind");
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    router.use_fake_openai_codex_native_vault_for_test();
    router.store_openai_codex_native_secret_for_test(
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        r#"{"refresh_token":"test-redacted"}"#,
    );
    (temp, listener, router)
}

fn complete_codex_bridge(router: &mut LocalApiRouter, responder_url: &str) {
    let started = route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account","stateSeed":"desktoplab-test"}"#,
    );
    let body = format!(
        r#"{{
            "pairingId":"{}",
            "pairingCode":"{}",
            "bridgeInstanceId":"desktoplab-macbook",
            "providerAccountLabel":"Simone OpenAI Codex",
            "localCredentialRef":"vault://desktoplab/external-backend/openai-codex/profile/simone",
            "responderUrl":"{}"
        }}"#,
        started["pairingId"].as_str().unwrap(),
        started["pairingCode"].as_str().unwrap(),
        responder_url
    );
    let _ = route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/complete",
        &body,
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
