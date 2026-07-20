use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn context_attachment_candidates_use_workspace_file_protection() {
    let (_fixture, mut router) = router_with_workspace();

    let attachments = route_json(
        &mut router,
        "GET",
        "/v1/workspaces/workspace.workspace/context-attachments",
        "",
    );

    assert_eq!(attachments["workspaceId"], "workspace.workspace");
    assert_eq!(attachments["attachments"][0]["path"], "README.md");
    assert_eq!(attachments["attachments"][0]["state"], "available");
    let protected = attachments["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|attachment| attachment["state"] == "unavailable")
        .map(|attachment| attachment["path"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(protected.contains(&".netrc"), "{protected:?}");
    assert!(protected.contains(&"certs/private.pem"), "{protected:?}");
    assert!(protected.contains(&"certs/service.key"), "{protected:?}");
    assert!(protected.contains(&".env"), "{protected:?}");
    assert!(
        attachments["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|attachment| attachment["state"] == "unavailable")
            .all(|attachment| attachment["disabledReason"] == "Protected local file.")
    );
}

#[test]
fn session_create_accepts_selected_context_paths() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Done");

    route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"Explain this repo","contextPaths":["README.md"]}"#,
    );
    let events = route_json(&mut router, "GET", "/v1/events/replay", "");

    assert!(
        events["frames"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| frame["payload"].to_string().contains("README.md"))
    );
}

#[test]
fn session_create_rejects_attachment_metadata_without_content() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Done");

    let response = router
        .route(
            "POST",
            "/v1/sessions",
            r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"Explain this file","externalAttachments":[{"name":"brief.txt","size":5,"mediaType":"text/plain"}]}"#,
        )
        .expect("session route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(
        response
            .body()
            .contains("EXTERNAL_ATTACHMENT_CONTENT_REQUIRED")
    );
}

#[test]
fn session_create_rejects_attachment_when_content_digest_does_not_match() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Done");

    let response = router
        .route(
            "POST",
            "/v1/sessions",
            r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"Explain this file","externalAttachments":[{"name":"brief.txt","size":5,"mediaType":"text/plain","contentText":"notes","contentSha256":"sha256:wrong"}]}"#,
        )
        .expect("session route should exist");

    assert_eq!(response.status(), "400 Bad Request");
    assert!(
        response
            .body()
            .contains("EXTERNAL_ATTACHMENT_DIGEST_MISMATCH")
    );
}

#[test]
fn session_create_records_content_bound_external_attachment_metadata() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Done");

    route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"Explain this file","externalAttachments":[{"name":"brief.txt","size":5,"mediaType":"text/plain","contentText":"notes","contentSha256":"sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309"}]}"#,
    );
    let events = route_json(&mut router, "GET", "/v1/events/replay", "");

    assert!(
        events["frames"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| frame["payload"].to_string().contains("brief.txt"))
    );
}

#[test]
fn session_create_uses_external_attachment_content_without_logging_raw_text() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Done");

    route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"Use attached file","externalAttachments":[{"name":"brief.txt","size":22,"mediaType":"text/plain","contentText":"TOKEN=secret\nsafe note","contentSha256":"sha256:91ba00ed80b2eb173cc3253d63e4ac029aa782d6de85f6b0424ec9b577c119a4"}]}"#,
    );
    let events = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = events["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(payloads.contains(r#""contentAttachments":1"#), "{payloads}");
    assert!(payloads.contains(r#""contentAttached":true"#), "{payloads}");
    assert!(!payloads.contains("TOKEN=secret"), "{payloads}");
    assert!(!payloads.contains("safe note"), "{payloads}");
}

#[test]
fn context_attachment_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_context_attachments.rs",
        include_str!("local_api_context_attachments.rs"),
        235,
    )
    .expect("context attachment test should stay focused");
}

fn router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("README.md"), "safe readme").unwrap();
    std::fs::write(workspace.join(".env"), "SECRET=hidden").unwrap();
    std::fs::write(workspace.join(".netrc"), "machine api login token").unwrap();
    std::fs::create_dir(workspace.join("certs")).unwrap();
    std::fs::write(workspace.join("certs").join("private.pem"), "pem").unwrap();
    std::fs::write(workspace.join("certs").join("service.key"), "key").unwrap();
    run_git(&workspace, &["init"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (fixture, router)
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

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
