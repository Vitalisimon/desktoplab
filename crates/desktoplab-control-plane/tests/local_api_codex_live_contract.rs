use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn codex_live_certification_blocks_without_vault_responder_egress_and_capabilities() {
    let mut router = LocalApiRouter::default();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/certify",
        r#"{"vaultRef":"local-file://token","responderUrl":"https://example.com","capabilities":[]}"#,
    );

    assert_eq!(blocked["status"], "blocked");
    assert_eq!(blocked["publicClaim"], "not_supported");
    assert!(
        blocked["blockedReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "credential_vault_ref_missing")
    );
    assert!(
        blocked["blockedReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "responder_must_be_loopback")
    );
}

#[test]
fn codex_live_certification_is_private_dev_only_after_all_boundaries_pass() {
    let mut router = LocalApiRouter::default();

    let certified = route_json(
        &mut router,
        "POST",
        "/v1/provider-bridges/openai-codex/certify",
        r#"{
            "vaultRef":"vault://desktoplab/external-backend/openai-codex/profile/simone",
            "responderUrl":"http://127.0.0.1:43109",
            "responderState":"healthy",
            "repositoryContextEgressApproval":"approved",
            "capabilities":[
                "account.consent",
                "credential.native_vault_ref",
                "event_stream.normalized",
                "tool_request.delegated"
            ]
        }"#,
    );

    assert_eq!(certified["status"], "certified_private_dev");
    assert_eq!(certified["certificationScope"], "private_dev_only");
    assert!(certified["blockedReasons"].as_array().unwrap().is_empty());
}

#[test]
fn codex_live_contract_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_codex_live_contract.rs",
        include_str!("local_api_codex_live_contract.rs"),
        110,
    )
    .expect("codex live contract tests should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
