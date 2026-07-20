use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn model_selected_write_supports_arbitrary_safe_filename_and_language() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Creating the model-selected target.","tool":"desktoplab.write_file","arguments":{"path":"artifacts/計算機.plan","content":"model selected content"}}"#,
    );

    let blocked = create_session(
        &mut router,
        "Necesito un artefacto nuevo para este proyecto",
    );

    assert_eq!(blocked["state"], "blocked");
    let approval_id = latest_approval_id(&mut router);
    assert_latest_approval(&mut router, "filesystem.write:artifacts/計算機.plan");
    resolve_approval(&mut router, &approval_id);
    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &serde_json::json!({
            "executionBackendId":"backend.ollama",
            "approvalId":approval_id
        })
        .to_string(),
    );
    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("artifacts/計算機.plan")).unwrap(),
        "model selected content"
    );
}

#[test]
fn model_selected_patch_replaces_exact_anchor_without_append_phrase_logic() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("CHANGELOG.md"), "old\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Applying an exact patch.","tool":"desktoplab.patch_file","arguments":{"path":"CHANGELOG.md","expected":"old","replacement":"old\nnew"}}"#,
    );

    let blocked = create_session(
        &mut router,
        "Aggiorna la documentazione come ritieni corretto",
    );
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("CHANGELOG.md")).unwrap(),
        "old\nnew\n"
    );
}

#[test]
fn model_selected_read_uses_structured_path_not_prompt_filename_parsing() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("notes.sql"), "select 1;").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Inspecting evidence.","tool":"desktoplab.read_file","arguments":{"path":"notes.sql"}}"#,
    );

    let completed = create_session(&mut router, "Spiegami cosa trovi di importante");

    assert_eq!(completed["state"], "completed");
    assert_tool_decision(&completed, "filesystem.read:notes.sql");
    assert!(completed["timeline"].to_string().contains("select 1;"));
}

#[test]
fn model_selected_clarification_blocks_without_inventing_a_target() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"I need a target.","tool":"desktoplab.clarify","arguments":{"question":"Which file should I edit?"}}"#,
    );

    let blocked = create_session(&mut router, "Fai la modifica necessaria");

    assert_eq!(blocked["state"], "blocked");
    assert!(
        blocked["timeline"]
            .to_string()
            .contains("clarification_required:Which file should I edit?")
    );
    assert!(
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn plain_assistant_prose_never_becomes_file_content() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Here is the file content, but no tool call.");

    let completed = create_session(&mut router, "Crea prose.md");

    assert_eq!(completed["state"], "completed");
    assert!(!workspace_root.join("prose.md").exists());
    assert!(
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn agent_file_intent_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_file_intent.rs",
        include_str!("local_api_agent_file_intent.rs"),
        250,
    )
    .expect("model-selected file routing test grew too large");
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":{},"executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            workspace_id,
            serde_json::to_string(prompt).unwrap()
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

fn assert_latest_approval(router: &mut LocalApiRouter, operation_id: &str) {
    let approvals = route_json(router, "GET", "/v1/approvals", "");
    assert_eq!(
        approvals["approvals"].as_array().unwrap().last().unwrap()["operationId"],
        operation_id
    );
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn assert_tool_decision(session: &Value, expected: &str) {
    assert!(session["timeline"].as_array().unwrap().iter().any(|event| {
        event["kind"] == "tool_decision"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    run_git(&workspace_root, &["init", "-b", "main"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").unwrap();
    let mut router = LocalApiRouter::default();
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
