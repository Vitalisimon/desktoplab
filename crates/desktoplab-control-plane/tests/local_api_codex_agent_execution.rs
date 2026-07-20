mod support;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::io::Write;
use std::net::TcpListener;
use support::accept_http_request;
use tempfile::tempdir;
use xtask::check_logical_line_limit;

#[test]
fn selected_codex_route_executes_through_codex_responder_not_local_ollama() {
    let temp = tempdir().expect("bridge dir should be created");
    let (endpoint, responder) = codex_responder();
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    router.use_fake_openai_codex_native_vault_for_test();
    router.store_openai_codex_native_secret_for_test(
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        r#"{"refresh_token":"test-redacted"}"#,
    );
    mark_setup_ready(&mut router);
    open_desktoplab_workspace(&mut router, temp.path());
    complete_codex_bridge(&mut router, &endpoint);
    route_json(
        &mut router,
        "POST",
        "/v1/routing/options/selection",
        r#"{"routeId":"route.external.codex"}"#,
    );

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.codex","initialPrompt":"Rispondi via Codex"}"#,
    );

    assert_eq!(created["executionBackendId"], "backend.codex");
    assert_eq!(created["state"], "completed");
    assert_eq!(
        created["timeline"][1]["message"],
        "Risposta dal responder Codex."
    );
    responder.join().expect("responder should finish");
}

#[test]
fn session_body_cannot_force_codex_when_local_route_is_selected() {
    let temp = tempdir().expect("workspace dir should be created");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    open_desktoplab_workspace(&mut router, temp.path());

    let response = router
        .route(
            "POST",
            "/v1/sessions",
            r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.codex","initialPrompt":"Bypass selected route"}"#,
        )
        .expect("session route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("ROUTE_BACKEND_MISMATCH"));
}

#[test]
fn local_api_codex_agent_execution_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_codex_agent_execution.rs",
        include_str!("local_api_codex_agent_execution.rs"),
        170,
    )
    .expect("codex agent execution test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/support/mod.rs",
        include_str!("support/mod.rs"),
        70,
    )
    .expect("shared HTTP test support should stay focused");
}

fn codex_responder() -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback responder should bind");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, request) = accept_http_request(&listener);
        assert!(request.contains("Rispondi via Codex"));
        assert!(request.contains("desktoplab.complete"));
        assert!(
            request.contains("vault://desktoplab/external-backend/openai-codex/profile/simone")
        );
        assert!(!request.contains("credentialPath"));
        assert!(!request.contains("refresh_token"));
        let body = serde_json::json!({
            "body":r#"{"tool":"desktoplab.complete","arguments":{"message":"Risposta dal responder Codex.","outcome":"answered","evidenceCallIds":[]}}"#,
            "providerResponseId":"codex_response_1"
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should be written");
    });
    (endpoint, handle)
}

fn complete_codex_bridge(router: &mut LocalApiRouter, responder_url: &str) {
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth_code_from_openai",
        "verifier_from_openai",
    );
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
    route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/complete",
        &body,
    );
}

fn open_desktoplab_workspace(router: &mut LocalApiRouter, root: &std::path::Path) {
    let workspace = root.join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should be created");
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace),
    );
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(router, "POST", "/v1/setup/complete", "{}");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
