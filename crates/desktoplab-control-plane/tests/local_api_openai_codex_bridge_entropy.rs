use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;

#[test]
fn openai_codex_pairing_ignores_client_seed_for_pairing_entropy() {
    let mut router = LocalApiRouter::default();
    authorize_device(&mut router, "device_auth_one", "AAAA-BBBB");
    let first = start_pairing(&mut router);
    authorize_device(&mut router, "device_auth_two", "CCCC-DDDD");
    let second = start_pairing(&mut router);

    assert_ne!(first["pairingId"], second["pairingId"]);
    assert_ne!(first["pairingCode"], second["pairingCode"]);
}

#[test]
fn openai_codex_bridge_entropy_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_openai_codex_bridge_entropy.rs",
        include_str!("local_api_openai_codex_bridge_entropy.rs"),
        80,
    )
    .expect("openai codex bridge entropy test should stay focused");
}

fn authorize_device(router: &mut LocalApiRouter, device_auth_id: &str, user_code: &str) {
    router.authorize_openai_codex_device_for_test(
        device_auth_id,
        user_code,
        "auth_code_from_openai",
        "verifier_from_openai",
    );
}

fn start_pairing(router: &mut LocalApiRouter) -> Value {
    route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account","stateSeed":"client-controlled-seed"}"#,
    )
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
