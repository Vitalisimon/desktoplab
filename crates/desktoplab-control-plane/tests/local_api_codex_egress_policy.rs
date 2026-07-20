mod support;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use support::accept_http_request;
use tempfile::tempdir;
use xtask::check_logical_line_limit;

#[test]
fn codex_route_blocks_repository_context_without_explicit_egress_approval() {
    let (temp, endpoint, responder) = paired_codex_responder(false);
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    mark_setup_ready(&mut router);
    open_desktoplab_workspace(&mut router, temp.path());
    complete_codex_bridge(&mut router, &endpoint);
    select_codex_route(&mut router);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &codex_session_with_context(None),
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        blocked["blockedReason"],
        "provider_egress_approval_required"
    );
    assert_eq!(blocked["approval"]["action"], "provider.egress");
    assert_eq!(
        blocked["approval"]["operationId"],
        "provider.openai:route.external.codex:workspace.desktoplab"
    );
    responder
        .join()
        .expect("responder should not receive a request");
}

#[test]
fn codex_route_sends_repository_context_after_matching_egress_approval() {
    let (temp, endpoint, responder) = paired_codex_context_responder();
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    mark_setup_ready(&mut router);
    open_desktoplab_workspace(&mut router, temp.path());
    complete_codex_bridge(&mut router, &endpoint);
    select_codex_route(&mut router);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &codex_session_with_context(None),
    );
    let approval_id = blocked["approval"]["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &codex_session_with_context(Some(&approval_id)),
    );

    assert_eq!(created["state"], "completed");
    responder.join().expect("responder should finish");
}

#[test]
fn codex_route_without_context_sends_only_user_prompt_to_responder() {
    let (temp, endpoint, responder) = paired_codex_responder(true);
    let mut router = LocalApiRouter::default().with_openai_codex_bridge_dir(temp.path());
    mark_setup_ready(&mut router);
    open_desktoplab_workspace(&mut router, temp.path());
    complete_codex_bridge(&mut router, &endpoint);
    select_codex_route(&mut router);

    let created = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.codex","initialPrompt":"Rispondi via Codex"}"#,
    );

    assert_eq!(created["executionBackendId"], "backend.codex");
    assert_eq!(created["state"], "completed");
    responder.join().expect("responder should finish");
}

#[test]
fn codex_egress_policy_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_codex_egress_policy.rs",
        include_str!("local_api_codex_egress_policy.rs"),
        270,
    )
    .expect("codex egress policy tests should stay focused");
}

fn codex_session_with_context(approval_id: Option<&str>) -> String {
    let approval = approval_id
        .map(|id| format!(r#","approvalId":"{id}""#))
        .unwrap_or_default();
    format!(
        r#"{{
            "workspaceId":"workspace.desktoplab",
            "executionBackendId":"backend.codex",
            "initialPrompt":"Inspect repo through Codex",
            "contextPaths":["README.md"]{approval}
        }}"#
    )
}

fn paired_codex_responder(
    expect_request: bool,
) -> (tempfile::TempDir, String, std::thread::JoinHandle<()>) {
    let temp = tempdir().expect("bridge dir should be created");
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback responder should bind");
    listener.set_nonblocking(!expect_request).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        if !expect_request {
            assert_no_nonempty_request(&listener);
            return;
        }
        let (mut stream, request) = accept_http_request(&listener);
        assert!(request.contains("Rispondi via Codex"));
        assert!(!request.contains("Repository context:"));
        assert!(!request.contains("README.md"));
        let body = canonical_completion("Risposta dal responder Codex.");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (temp, endpoint, handle)
}

fn paired_codex_context_responder() -> (tempfile::TempDir, String, std::thread::JoinHandle<()>) {
    let temp = tempdir().expect("bridge dir should be created");
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback responder should bind");
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, request) = accept_http_request(&listener);
        assert!(request.contains("Inspect repo through Codex"));
        assert!(request.contains("Repository context:"));
        let body = canonical_completion("Codex read approved repository context.");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (temp, endpoint, handle)
}

fn canonical_completion(message: &str) -> String {
    serde_json::json!({
        "body":serde_json::json!({
            "tool":"desktoplab.complete",
            "arguments":{"message":message,"outcome":"answered","evidenceCallIds":[]}
        }).to_string()
    })
    .to_string()
}

fn assert_no_nonempty_request(listener: &TcpListener) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
    while std::time::Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut request = [0_u8; 256];
                let read = stream.read(&mut request).unwrap_or_default();
                assert_eq!(read, 0, "responder must not receive repository context");
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
}

fn open_desktoplab_workspace(router: &mut LocalApiRouter, root: &std::path::Path) {
    let workspace = root.join("desktoplab");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    run_git(&workspace, &["init", "-b", "main"]);
    std::fs::write(
        workspace.join("README.md"),
        "DesktopLab workspace context\n",
    )
    .unwrap();
    run_git(&workspace, &["add", "README.md"]);
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(output.status.success(), "git {:?} failed", args);
}

fn complete_codex_bridge(router: &mut LocalApiRouter, responder_url: &str) {
    router.use_fake_openai_codex_native_vault_for_test();
    router.store_openai_codex_native_secret_for_test(
        "vault://desktoplab/external-backend/openai-codex/profile/simone",
        r#"{"refresh_token":"test-redacted"}"#,
    );
    router.authorize_openai_codex_device_for_test(
        "device_auth_test",
        "ABCD-EFGH",
        "auth",
        "verifier",
    );
    let started = route_json(
        router,
        "POST",
        "/v1/provider-bridges/openai-codex/pairing/start",
        r#"{"accountMode":"subscription_account","stateSeed":"desktoplab-test"}"#,
    );
    let body = format!(
        r#"{{"pairingId":"{}","pairingCode":"{}","bridgeInstanceId":"desktoplab","providerAccountLabel":"Codex","localCredentialRef":"vault://desktoplab/external-backend/openai-codex/profile/simone","responderUrl":"{}"}}"#,
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

fn select_codex_route(router: &mut LocalApiRouter) {
    route_json(
        router,
        "POST",
        "/v1/routing/options/selection",
        r#"{"routeId":"route.external.codex"}"#,
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
