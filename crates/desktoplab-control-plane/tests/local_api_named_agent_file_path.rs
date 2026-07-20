use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn named_markdown_creation_prompt_writes_the_requested_file_after_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo prova.md.","desktoplabAction":{"kind":"create_file","path":"prova.md","content":"# Agenti AI\n"}}"##,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"crea un nuovo file: prova.md Scrivici un paper sugli agenti ai"}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    let approval_id = listed["approvals"][0]["approvalId"].as_str().unwrap();
    assert_eq!(
        listed["approvals"][0]["operationId"],
        "filesystem.write:prova.md"
    );
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("prova.md")).unwrap(),
        "# Agenti AI\n"
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
}

#[test]
fn named_agent_file_path_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_named_agent_file_path.rs",
        include_str!("local_api_named_agent_file_path.rs"),
        120,
    )
    .expect("named agent file path test should stay focused");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    (fixture, workspace_root, router)
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.enable_test_controls_for_dev_server();
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
    assert!(output.status.success(), "git {:?} failed", args);
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
