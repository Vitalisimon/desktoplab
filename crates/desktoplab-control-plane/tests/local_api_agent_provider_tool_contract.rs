use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn provider_schema_read_file_records_real_file_observation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("notes.md"), "# Notes\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Leggo notes.md.","tool":"desktoplab.read_file","arguments":{"path":"notes.md"}}"#,
    );

    let completed = create_session(&mut router, "このリポジトリを確認してください");

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "Read notes.md:");
    assert_timeline_contains(&completed, "# Notes");
}

#[test]
fn provider_short_name_is_rejected_instead_of_silently_rewritten() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("README.md"),
        "# Demo\nImportant module.\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"read_file","arguments":{"path":"README.md"}}"#,
    );

    let completed = create_session(&mut router, "leggi README.md e dimmi cosa contiene");

    assert_eq!(completed["state"], "failed");
    assert_timeline_contains(&completed, "malformed structured file action");
    assert_transcript_excludes(&completed, r#""name":"read_file""#);
}

#[test]
fn concatenated_unknown_tool_aliases_fail_closed() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::create_dir_all(workspace_root.join("src")).unwrap();
    std::fs::write(
        workspace_root.join("README.md"),
        "# Demo\nImportant module.\n",
    )
    .unwrap();
    std::fs::write(workspace_root.join("AGENTS.md"), "# Agent notes\n").unwrap();
    std::fs::write(workspace_root.join("src/lib.rs"), "pub fn core() {}\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"legacy_read_file","arguments":{"path":"README.md"}} {"name":"legacy_read_file","arguments":{"path":"src/lib.rs"}}"#,
    );

    let completed = create_session(&mut router, "leggi questa repo e dimmi i moduli");

    assert_eq!(completed["state"], "failed");
    assert_timeline_contains(&completed, "malformed structured file action");
    assert_transcript_excludes(&completed, r#""name":"read_file""#);
}

#[test]
fn canonical_patch_name_uses_the_real_approval_and_executor() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let workspace_id =
        route_json(&mut router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    std::fs::write(
        workspace_root.join("calculator.js"),
        "return left - right;\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"calculator.js","expected":"left - right","replacement":"left + right"}}"#,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":"repair calculator.js"}}"#,
            serde_json::to_string(&workspace_id).unwrap()
        ),
    );

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_eq!(blocked["pendingApprovals"][0]["action"], "filesystem.write");
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "filesystem.patch:calculator.js"
    );
    assert_eq!(
        blocked["pendingApprovals"][0]["title"],
        "Patch calculator.js"
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("calculator.js")).unwrap(),
        "return left - right;\n"
    );

    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    resolve_approval(&mut router, approval_id);
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed");
    assert_timeline_contains(&completed["session"], "filesystem.patch:calculator.js");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("calculator.js")).unwrap(),
        "return left + right;\n"
    );
}

#[test]
fn provider_schema_list_files_records_real_workspace_entries() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::create_dir_all(workspace_root.join("src")).unwrap();
    std::fs::write(workspace_root.join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Elenco i file.","tool":"desktoplab.list_files","arguments":{"path":"."}}"#,
    );

    let completed = create_session(&mut router, "elenca i file");

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "Workspace files:");
    assert_timeline_contains(&completed, "src/lib.rs");
}

#[test]
fn provider_schema_search_text_records_real_matches() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("notes.md"), "composer lives here\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Cerco composer.","tool":"desktoplab.search_text","arguments":{"query":"composer","path":"."}}"#,
    );

    let completed = create_session(&mut router, "cerca composer");

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "Search results for `composer`:");
    assert_timeline_contains(&completed, "notes.md");
}

#[test]
fn provider_schema_create_checkpoint_records_real_git_checkpoint() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Creo checkpoint.","tool":"desktoplab.create_checkpoint","arguments":{"label":"before risky work"}}"#,
    );

    let completed = create_session(&mut router, "crea checkpoint");

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "Checkpoint ready:");
}

#[test]
fn provider_schema_clarify_blocks_with_model_question() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Mi serve un dettaglio.","tool":"desktoplab.clarify","arguments":{"question":"Quale file devo modificare?"}}"#,
    );

    let blocked = create_session(&mut router, "continua con la domanda del modello");

    assert_eq!(blocked["state"], "blocked");
    assert_timeline_contains(
        &blocked,
        "clarification_required:Quale file devo modificare?",
    );
}

#[test]
fn provider_schema_run_terminal_requires_approval_then_executes() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let command = write_stdout("provider-terminal-ok");
    router.complete_agent_backend_for_test(
        &serde_json::json!({
            "assistantMessage": "Run terminal validation.",
            "tool": "desktoplab.run_terminal",
            "arguments": { "command": command }
        })
        .to_string(),
    );

    let blocked = create_session(&mut router, "run terminal validation");
    assert_eq!(blocked["state"], "blocked");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed");
    assert_timeline_contains(&completed, "provider-terminal-ok");
}

#[test]
fn provider_terminal_workspace_root_is_scoped_to_the_active_workspace() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let command = write_stdout("provider-terminal-root-ok");
    router.complete_agent_backend_for_test(
        &serde_json::json!({
            "assistantMessage": "Run terminal validation.",
            "tool": "desktoplab.run_terminal",
            "arguments": { "command": command, "cwd": "/" }
        })
        .to_string(),
    );

    let blocked = create_session(&mut router, "run terminal validation at workspace root");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "provider-terminal-root-ok");
}

#[test]
fn provider_schema_commit_changes_requires_approval_then_commits() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Changed\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Committo.","tool":"desktoplab.commit_changes","arguments":{"message":"docs: provider commit"}}"#,
    );

    let blocked = create_session(&mut router, "committa le modifiche");
    assert_eq!(blocked["state"], "blocked", "{blocked}");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed", "{completed}");
    let message = git_stdout(&workspace_root, &["log", "-1", "--pretty=%B"]);
    assert!(message.starts_with("docs: provider commit\n"), "{message}");
}

#[test]
fn provider_tool_contract_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_provider_tool_contract.rs",
        include_str!("local_api_agent_provider_tool_contract.rs"),
        360,
    )
    .expect("provider tool contract test should stay focused");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(&workspace_id).unwrap(),
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

#[cfg(not(windows))]
fn write_stdout(value: &str) -> String {
    format!("printf '{value}'")
}

#[cfg(windows)]
fn write_stdout(value: &str) -> String {
    format!("[Console]::Write('{value}')")
}

fn continue_approval(router: &mut LocalApiRouter, blocked: &Value, approval_id: &str) -> Value {
    route_json(
        router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    )
}

fn latest_approval_id(router: &mut LocalApiRouter) -> String {
    route_json(router, "GET", "/v1/approvals", "")["approvals"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["approvalId"]
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

fn assert_transcript_excludes(session: &Value, unexpected: &str) {
    let transcript = serde_json::to_string(&session["transcript"]).unwrap();
    assert!(
        !transcript.contains(unexpected),
        "unexpected {unexpected}: {transcript}"
    );
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace_root, &["add", "."]);
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
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
