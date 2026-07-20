use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn filesystem_write_agent_step_creates_resumable_approval_record() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let workspace_id =
        route_json(&mut router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo guida-calcolatrice.md.","desktoplabAction":{"kind":"create_file","path":"guida-calcolatrice.md","content":"# Calcolatrice semplice\n"}}"##,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Crea un file .md per una calcolatrice semplice"}}"#
        ),
    );

    assert_eq!(blocked["state"], "blocked");
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());

    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    let approval_id = listed["approvals"][0]["approvalId"].as_str().unwrap();
    assert_eq!(listed["approvals"][0]["sessionId"], blocked["sessionId"]);
    assert_eq!(listed["approvals"][0]["action"], "filesystem.write");
    assert!(
        listed["approvals"][0]["payloadHash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(
        listed["approvals"][0]["operationId"],
        "filesystem.write:guida-calcolatrice.md"
    );
    assert_eq!(blocked["pendingApprovals"][0]["approvalId"], approval_id);

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert_eq!(
        workspace["session"]["pendingApprovals"][0]["approvalId"],
        approval_id
    );

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    router.complete_agent_backend_for_test("contenuto cambiato dopo approvazione\n");
    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("guida-calcolatrice.md")).unwrap(),
        "# Calcolatrice semplice\n"
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
    let replayed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(replayed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("guida-calcolatrice.md")).unwrap(),
        "# Calcolatrice semplice\n"
    );
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"][0]["consumed"], true);
    assert_transcript_excludes(&completed, "waiting for approval");
}

#[test]
fn italian_create_file_prompt_without_target_requests_clarification() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let workspace_id =
        route_json(&mut router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Serve una scelta.","tool":"desktoplab.clarify","arguments":{"question":"Quale file devo creare?"}}"#,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Prova a creare il file"}}"#
        ),
    );

    assert_eq!(blocked["state"], "blocked");
    assert!(
        blocked["timeline"]
            .to_string()
            .contains("clarification_required:Quale file devo creare?"),
        "{blocked}"
    );
    assert_eq!(
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());

    let context = router
        .workspace_context_for_session_prompt_for_test(
            &workspace_id,
            blocked["sessionId"].as_str().unwrap(),
            "guida.md",
        )
        .expect("clarification follow-up context should exist");
    assert!(context.contains("user: Prova a creare il file"));
    assert!(context.contains("assistant: Quale file devo creare?"));
}

#[test]
fn ambiguous_create_file_prompt_blocks_for_filename_clarification() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Need target.","tool":"desktoplab.clarify","arguments":{"question":"Which file should I create?"}}"#,
    );

    let blocked = create_session(&mut router, "crea un file");

    assert_eq!(blocked["state"], "blocked");
    assert!(
        blocked["timeline"]
            .to_string()
            .contains("clarification_required:Which file should I create?"),
        "{blocked}"
    );
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"].as_array().unwrap().len(), 0);
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
}

#[test]
fn unnamed_document_prompt_uses_backend_action_path_without_legacy_fallback() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo git-reference.md.","desktoplabAction":{"kind":"create_file","path":"git-reference.md","content":"# Funzioni Git\n"}}"##,
    );

    let blocked = create_session(
        &mut router,
        "Crea un documento, in cui descrivi le funzioni git",
    );

    assert_eq!(blocked["state"], "blocked");
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["approvals"][0]["operationId"],
        "filesystem.write:git-reference.md"
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
}

#[test]
fn approved_write_then_read_prompt_records_readback_observation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo e leggo il file.","desktoplabAction":{"kind":"create_file","path":"paper.md","content":"# Agenti AI\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea paper.md e poi leggilo");
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
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
        &continuation_body(&approval_id),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("paper.md")).unwrap(),
        "# Agenti AI\n"
    );
    assert_tool_decision(&completed, "filesystem.write:paper.md");
    assert_tool_decision(&completed, "filesystem.read:paper.md");
    assert_assistant_message_contains(&completed, "# Agenti AI");
}

#[test]
fn approved_patch_file_preserves_unrelated_content_and_records_diff() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("notes.md"), "alpha\nbeta\ngamma\n").unwrap();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Patch notes.md.","desktoplabAction":{"kind":"patch_file","path":"notes.md","expected":"beta\n","replacement":"beta updated\n"}}"##,
    );

    let blocked = create_session(&mut router, "modifica notes.md aggiornando beta");
    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
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
        &continuation_body(&approval_id),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("notes.md")).unwrap(),
        "alpha\nbeta updated\ngamma\n"
    );
    assert_tool_decision(&completed, "filesystem.patch:notes.md");
    assert_assistant_message_contains(&completed, "diff --git");
    assert_assistant_message_contains(&completed, "+beta updated");
}

#[test]
fn malformed_structured_file_action_blocks_without_approval_or_write() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo il file.","desktoplabAction":{"kind":"create_file","path":"../other.md","content":"# Wrong\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea notes.md");

    assert!(matches!(
        blocked["state"].as_str(),
        Some("blocked" | "failed")
    ));
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(listed["approvals"].as_array().unwrap().len(), 0);
    assert!(!workspace_root.join("notes.md").exists());
    assert!(!workspace_root.join("other.md").exists());
}

#[test]
fn agent_tool_approval_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_tool_approvals.rs",
        include_str!("local_api_agent_tool_approvals.rs"),
        410,
    )
    .expect("agent tool approval test should stay focused");
}

fn assert_tool_decision(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "tool_decision"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &serde_json::json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":prompt
        })
        .to_string(),
    )
}

fn continuation_body(approval_id: &str) -> String {
    serde_json::json!({
        "executionBackendId":"backend.ollama",
        "prompt":"Continue approved action",
        "approvalId":approval_id
    })
    .to_string()
}

fn assert_assistant_message_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "assistant"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
