use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn agent_file_observations_are_redacted_in_transcript_and_details() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("README.md"),
        "token=sk-read-secret\nsafe docs\n",
    )
    .unwrap();
    std::fs::write(
        workspace_root.join(".env"),
        "OPENAI_API_KEY=sk-env-secret\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Leggo README.","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
    );

    let session = create_session(&mut router, "leggi README.md");
    let serialized = serde_json::to_string(&session).unwrap();

    assert_eq!(session["state"], "completed");
    assert!(!serialized.contains("sk-read-secret"));
    assert!(!serialized.contains("sk-env-secret"));
    assert!(serialized.contains("[REDACTED]"));
    assert!(serialized.contains(r#""redactionSource":"agent.observation""#));
}

#[test]
fn agent_terminal_stdout_and_git_diff_are_redacted_before_display_or_export() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "safe\n").unwrap();
    commit_fixture_files(&workspace_root);

    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Eseguo controllo.","tool":"desktoplab.run_terminal","arguments":{"command":"printf 'token=sk-terminal-secret\n'","reason":"redaction regression"}}"#,
    );
    let terminal = create_session(&mut router, "esegui controllo");
    let terminal_approval = terminal["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    approve(&mut router, terminal_approval);
    let terminal_done = continue_approval(&mut router, &terminal, terminal_approval);
    let terminal_json = serde_json::to_string(&terminal_done).unwrap();
    assert!(!terminal_json.contains("sk-terminal-secret"));
    assert!(terminal_json.contains("redaction_status=redacted"));
    assert!(terminal_json.contains("terminal.stdout"));

    std::fs::write(workspace_root.join("README.md"), "API_KEY=sk-diff-secret\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Mostro diff.","tool":"desktoplab.git_diff","arguments":{}}"#,
    );
    let diff = create_session(&mut router, "mostrami il diff");
    let diff_json = serde_json::to_string(&diff).unwrap();
    assert!(!diff_json.contains("sk-diff-secret"));
    assert!(diff_json.contains("Git diff: redacted=true"));
    assert!(diff_json.contains("[REDACTED]"));

    let export = route_json(&mut router, "GET", "/v1/diagnostics/export", "");
    let export_json = serde_json::to_string(&export).unwrap();
    assert!(!export_json.contains("sk-terminal-secret"));
    assert!(!export_json.contains("sk-diff-secret"));
    assert_eq!(export["redaction"]["rawToolOutputIncluded"], false);
    assert_eq!(export["redaction"]["secretsIncluded"], false);
}

#[test]
fn local_api_agent_redaction_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_redaction.rs",
        include_str!("local_api_agent_redaction.rs"),
        220,
    )
    .expect("agent redaction tests should stay focused");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    router.set_host_memory_gb_for_test(32);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
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

fn approve(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn continue_approval(router: &mut LocalApiRouter, session: &Value, approval_id: &str) -> Value {
    route_json(
        router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            session["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"continue","approvalId":"{approval_id}"}}"#
        ),
    )
}

fn commit_fixture_files(root: &std::path::Path) {
    run_git(root, &["add", "."]);
    run_git(
        root,
        &[
            "-c",
            "user.name=DesktopLab Test",
            "-c",
            "user.email=desktoplab@example.local",
            "commit",
            "-m",
            "baseline",
        ],
    );
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
