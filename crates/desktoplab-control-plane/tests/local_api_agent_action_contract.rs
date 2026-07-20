use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn provider_schema_write_file_action_creates_resumable_filesystem_write() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo provider.md.","tool":"desktoplab.write_file","arguments":{"path":"provider.md","content":"# Provider action\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea provider.md");
    assert_eq!(blocked["state"], "blocked");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("provider.md")).unwrap(),
        "# Provider action\n"
    );
}

#[test]
fn provider_schema_patch_file_action_applies_expected_replacement() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("notes.md"), "alpha\nbeta\ngamma\n").unwrap();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Patch notes.md.","tool":"desktoplab.patch_file","arguments":{"path":"notes.md","expected":"beta\n","replacement":"beta native\n"}}"##,
    );

    let blocked = create_session(&mut router, "aggiorna notes.md");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("notes.md")).unwrap(),
        "alpha\nbeta native\ngamma\n"
    );
}

#[test]
fn git_commit_pending_action_resumes_into_real_commit() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    configure_git_identity(&workspace_root);
    std::fs::write(workspace_root.join("README.md"), "# Changed\n").unwrap();
    router.complete_agent_backend_for_test("Committo la modifica dopo approvazione.");

    let blocked = create_session_with(
        &mut router,
        json!({"initialPrompt":"committa","plannedTool":"git.commit","message":"docs: update readme"}),
    );
    assert_eq!(blocked["state"], "blocked");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed", "{completed}");
    let message = git_stdout(&workspace_root, &["log", "-1", "--pretty=%B"]);
    assert!(message.starts_with("docs: update readme\n"), "{message}");
    assert!(
        message.contains("DesktopLab-Session: session.1"),
        "{message}"
    );
}

#[test]
fn git_commit_pending_action_rechecks_worktree_fingerprint_before_consumption() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    configure_git_identity(&workspace_root);
    std::fs::write(workspace_root.join("README.md"), "# Approved change\n").unwrap();
    router.complete_agent_backend_for_test("Committo la modifica dopo approvazione.");

    let blocked = create_session_with(
        &mut router,
        json!({"initialPrompt":"committa","plannedTool":"git.commit","message":"docs: approved change"}),
    );
    assert_eq!(blocked["state"], "blocked");
    let approval_id = latest_approval_id(&mut router);
    std::fs::write(
        workspace_root.join("README.md"),
        "# Changed before approval consumption\n",
    )
    .unwrap();
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "failed", "{completed}");
    assert_timeline_contains(&completed, "working_tree_changed_after_approval");
    let log = git_stdout(&workspace_root, &["log", "--oneline"]);
    assert!(!log.contains("docs: approved change"), "{log}");
}

#[test]
fn git_push_pending_action_resumes_through_git_executor() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    configure_git_identity(&workspace_root);
    let remote = TempDir::new().unwrap();
    run_git(remote.path(), &["init", "--bare"]);
    run_git(
        &workspace_root,
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    router.complete_agent_backend_for_test("Push approvato.");

    let blocked = create_session_with(
        &mut router,
        json!({"initialPrompt":"pusha","plannedTool":"git.push","remote":"origin","branch":"main"}),
    );
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        git_stdout(remote.path(), &["rev-parse", "--verify", "main"]).len(),
        41
    );
}

#[test]
fn planned_actions_without_required_fields_block_instead_of_defaulting() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Non ho parametri sufficienti.");

    let terminal = create_session_with(
        &mut router,
        json!({"initialPrompt":"esegui","plannedTool":"terminal.command"}),
    );
    let tests = create_session_with(
        &mut router,
        json!({"initialPrompt":"test","plannedTool":"desktoplab.run_tests"}),
    );
    let commit = create_session_with(
        &mut router,
        json!({"initialPrompt":"commit","plannedTool":"git.commit"}),
    );

    assert_blocked_without_approval(&mut router, &terminal, "clarification_required:command");
    assert_blocked_without_approval(&mut router, &tests, "clarification_required:test_command");
    assert_blocked_without_approval(
        &mut router,
        &commit,
        "clarification_required:commit_message",
    );
}

#[test]
fn failing_terminal_and_test_preserve_evidence_for_the_agent() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Eseguo il comando richiesto.");

    let terminal = create_session_with(
        &mut router,
        json!({"initialPrompt":"fallisci","plannedTool":"terminal.command","command":"false"}),
    );
    let terminal_approval = latest_approval_id(&mut router);
    resolve_approval(&mut router, &terminal_approval);
    let terminal_completed = continue_approval(&mut router, &terminal, &terminal_approval);

    router.complete_agent_backend_for_test("Eseguo i test richiesti.");
    let tests = create_session_with(
        &mut router,
        json!({"initialPrompt":"test fallito","plannedTool":"desktoplab.run_tests","command":"pwd; false","reason":"prove failure handling"}),
    );
    let test_approval = latest_approval_id(&mut router);
    resolve_approval(&mut router, &test_approval);
    let test_completed = continue_approval(&mut router, &tests, &test_approval);

    assert_ne!(terminal_completed["state"], "failed");
    assert_timeline_contains(&terminal_completed, "status=exited:1");
    assert_ne!(test_completed["state"], "failed");
    assert_timeline_contains(&test_completed, "finished with status Exited(1)");
    assert!(
        !test_completed.to_string().contains(
            workspace_root
                .to_str()
                .expect("workspace path must be UTF-8")
        )
    );
}

#[test]
fn agent_action_contract_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_action_contract.rs",
        include_str!("local_api_agent_action_contract.rs"),
        380,
    )
    .expect("agent action contract test should stay focused");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    create_session_with(router, json!({"initialPrompt":prompt}))
}

fn create_session_with(router: &mut LocalApiRouter, mut body: Value) -> Value {
    body["workspaceId"] = Value::String(workspace_id(router));
    body["executionBackendId"] = Value::String("backend.ollama".to_string());
    route_json(router, "POST", "/v1/sessions", &body.to_string())
}

fn continue_approval(router: &mut LocalApiRouter, blocked: &Value, approval_id: &str) -> Value {
    route_json(
        router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &json!({"executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":approval_id}).to_string(),
    )
}

fn workspace_id(router: &mut LocalApiRouter) -> String {
    route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn latest_approval_id(router: &mut LocalApiRouter) -> String {
    let listed = route_json(router, "GET", "/v1/approvals", "");
    listed["approvals"].as_array().unwrap().last().unwrap()["approvalId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn assert_blocked_without_approval(router: &mut LocalApiRouter, session: &Value, expected: &str) {
    assert_eq!(session["state"], "blocked");
    assert!(
        session["timeline"].to_string().contains(expected),
        "{session}"
    );
    assert_eq!(
        route_json(router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| event["message"]
            .as_str()
            .is_some_and(|message| message.contains(expected))),
        "missing {expected}: {session}"
    );
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace_root, &["add", "."]);
    configure_git_identity(&workspace_root);
    run_git(&workspace_root, &["commit", "-m", "initial"]);
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

fn configure_git_identity(root: &std::path::Path) {
    run_git(root, &["config", "user.email", "desktoplab@example.test"]);
    run_git(root, &["config", "user.name", "DesktopLab Test"]);
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

fn git_stdout(root: &std::path::Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
