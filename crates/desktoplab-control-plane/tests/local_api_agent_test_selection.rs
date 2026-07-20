use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn model_selected_project_test_command_requires_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .unwrap();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#,
    ]);

    let blocked = create_session(&mut router, "esegui i test mirati");
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(approvals["approvals"][0]["action"], "test.run");
    assert_eq!(
        approvals["approvals"][0]["operationId"],
        "test.run:npm test"
    );
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "test.run:npm test"
    );
}

#[test]
fn model_can_request_clarification_for_ambiguous_project_commands() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .unwrap();
    std::fs::write(
        workspace_root.join("Cargo.toml"),
        "[package]\nname='demo'\n",
    )
    .unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.clarify","arguments":{"question":"I found npm test and cargo test. Which test command should I run?","blockedOn":"desktoplab.run_tests"}}"#,
    ]);

    let blocked = create_session(&mut router, "esegui i test mirati");
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        blocked["blockedReason"],
        "clarification_required:I found npm test and cargo test. Which test command should I run?"
    );
    assert!(approvals["approvals"].as_array().unwrap().is_empty());
    assert_timeline_contains(
        &blocked,
        "clarification_required:I found npm test and cargo test",
    );
    assert_timeline_contains(&blocked, "npm test");
    assert_timeline_contains(&blocked, "cargo test");
}

#[test]
fn planning_prompt_that_mentions_test_does_not_select_project_command() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Use tests first, then make a focused change.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);

    let completed = create_session(&mut router, "Inspect repo and propose first test");
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");

    assert_eq!(completed["state"], "completed");
    assert!(approvals["approvals"].as_array().unwrap().is_empty());
    assert_timeline_contains(&completed, "Use tests first");
}

#[test]
fn multi_step_repair_prompt_does_not_short_circuit_to_test_command() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .unwrap();
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.complete","arguments":{"message":"The multi-step request remains in the agent loop.","outcome":"answered","evidenceCallIds":[]}}"#,
    ]);

    let completed = create_session(
        &mut router,
        "Leggi README.md, correggi answer ed esegui i test mirati",
    );
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");

    assert_eq!(completed["state"], "completed");
    assert!(approvals["approvals"].as_array().unwrap().is_empty());
    assert_timeline_contains(&completed, "multi-step request");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["message"]
            .as_str()
            .is_some_and(|message| message.contains(expected))
    }));
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
