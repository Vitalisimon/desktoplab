use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn structured_file_action_writes_content_not_model_envelope() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo il file.","desktoplabAction":{"kind":"create_file","path":"notes.md","content":"# Note\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea notes.md");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("notes.md")).unwrap(),
        "# Note\n"
    );
}

#[test]
fn fenced_structured_file_action_writes_content_and_hides_envelope() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"```json
{"assistantMessage":"File created successfully","desktoplabAction":{"kind":"create_file","path":"notes.md","content":"# Git Usage Notes\n\nLonger notes.\n"}}
```"##,
    );

    let blocked = create_session(&mut router, "crea notes.md con le funzioni git");
    assert_assistant_message_contains(&blocked, "File created successfully");
    assert_no_assistant_message_contains(&blocked, "desktoplabAction");
    let approval_id = latest_approval_id(&mut router);
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("notes.md")).unwrap(),
        "# Git Usage Notes\n\nLonger notes.\n"
    );
}

#[test]
fn targetless_document_prompt_uses_backend_structured_action_path() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo manuale-tastiera.md.","desktoplabAction":{"kind":"create_file","path":"manuale-tastiera.md","content":"# Scorciatoie\n"}}"##,
    );

    let blocked = create_session(
        &mut router,
        "prova a creare un nuovo file doc, in cui descrivi le scorciatoie da tastiera",
    );
    let approval_id = latest_approval_id(&mut router);
    let listed = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(
        listed["approvals"][0]["operationId"],
        "filesystem.write:manuale-tastiera.md"
    );
    resolve_approval(&mut router, &approval_id);
    let completed = continue_approval(&mut router, &blocked, &approval_id);

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("manuale-tastiera.md")).unwrap(),
        "# Scorciatoie\n"
    );
    assert!(!workspace_root.join("scorciatoie-da-tastiera.md").exists());
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
}

#[test]
fn structured_action_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_structured_actions.rs",
        include_str!("local_api_agent_structured_actions.rs"),
        280,
    )
    .expect("structured action tests should stay focused");
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

fn continue_approval(router: &mut LocalApiRouter, blocked: &Value, approval_id: &str) -> Value {
    route_json(
        router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &serde_json::json!({
            "executionBackendId":"backend.ollama",
            "prompt":"Continue approved action",
            "approvalId":approval_id
        })
        .to_string(),
    )
}

fn latest_approval_id(router: &mut LocalApiRouter) -> String {
    let listed = route_json(router, "GET", "/v1/approvals", "");
    listed["approvals"][0]["approvalId"]
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

fn assert_assistant_message_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["kind"] == "assistant"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
    }));
}

fn assert_no_assistant_message_contains(session: &Value, unexpected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(!timeline.iter().any(|event| {
        event["kind"] == "assistant"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(unexpected))
    }));
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
