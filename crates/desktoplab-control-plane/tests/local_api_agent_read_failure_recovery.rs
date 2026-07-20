use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn missing_file_observation_returns_to_the_agent_for_a_grounded_answer() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"missing.md"}}"#,
        r#"{"name":"desktoplab.complete","arguments":{"message":"missing.md does not exist in this repository."}}"#,
    ]);

    let completed = create_session(&mut router, "read missing.md and report what happens");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(
        &completed,
        "state=failed source=filesystem.read canonical=desktoplab.read_file tool=filesystem.read:missing.md",
    );
    assert_timeline_contains(&completed, "executor_reason=read_failed");
    assert_timeline_contains(&completed, "missing.md does not exist");
}

#[test]
fn path_escape_observation_is_recoverable_without_disclosing_external_content() {
    let (fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        fixture.path().join("outside-secret.txt"),
        "never disclose this value",
    )
    .expect("outside fixture should exist");
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"../outside-secret.txt"}}"#,
        r#"{"name":"desktoplab.complete","arguments":{"message":"DesktopLab blocked access outside this repository."}}"#,
    ]);

    let completed = create_session(&mut router, "read ../outside-secret.txt");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "executor_reason=path_escape");
    assert_timeline_contains(&completed, "blocked access outside this repository");
    assert!(!completed.to_string().contains("never disclose this value"));
    assert!(!workspace_root.join("outside-secret.txt").exists());
}

#[test]
fn read_failure_recovery_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_read_failure_recovery.rs",
        include_str!("local_api_agent_read_failure_recovery.rs"),
        170,
    )
    .expect("read failure recovery tests should stay focused");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    (fixture, workspace_root, router)
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    assert!(
        session["timeline"].as_array().unwrap().iter().any(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
        }),
        "missing {expected}: {session}"
    );
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
        .expect("route should exist");
    serde_json::from_str(response.body()).expect("response should be JSON")
}
