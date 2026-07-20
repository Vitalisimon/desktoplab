use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn workspace_file_preview_redacts_secret_like_text() {
    let (_fixture, mut router) = router_with_workspace();

    let preview = route_json(
        &mut router,
        "GET",
        "/v1/workspaces/workspace.workspace/files/preview?path=notes.txt",
        "",
    );

    assert_eq!(preview["state"], "text");
    assert!(preview["text"].as_str().unwrap().contains("visible note"));
    assert!(
        preview["text"]
            .as_str()
            .unwrap()
            .contains("[REDACTED_SECRET]")
    );
    assert!(
        !preview["text"]
            .as_str()
            .unwrap()
            .contains("sk-preview-secret")
    );
}

#[test]
fn file_preview_redaction_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_file_preview_redaction.rs",
        include_str!("local_api_file_preview_redaction.rs"),
        120,
    )
    .expect("file preview redaction test should stay focused");
}

fn router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(
        workspace.join("notes.txt"),
        "visible note\napi_key=sk-preview-secret\n",
    )
    .unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
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

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
