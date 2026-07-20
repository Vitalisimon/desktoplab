use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

const CALCULATOR_PROMPT: &str =
    "prova a creare un file .md in cui spieghi come creeresti una app calcolatrice semplice";

#[test]
fn targetless_plain_prose_never_becomes_a_file_action() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("# App calcolatrice semplice\n");

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &session_body(CALCULATOR_PROMPT),
    );

    assert_eq!(completed["state"], "completed");
    assert_no_tool_decision_contains(&completed, "filesystem.read:README.md");
    assert_eq!(
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
    assert!(!workspace_root.join("README.md").exists());
}

#[test]
fn structured_markdown_creation_writes_after_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo calcolatrice.md.","desktoplabAction":{"kind":"create_file","path":"calcolatrice.md","content":"# App calcolatrice semplice\n"}}"##,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &session_body(CALCULATOR_PROMPT),
    );
    assert_eq!(blocked["state"], "blocked");

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
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("calcolatrice.md")).unwrap(),
        "# App calcolatrice semplice\n"
    );
    assert!(!workspace_root.join("DESKTOPLAB_AGENT_NOTES.md").exists());
    assert!(!workspace_root.join("README.md").exists());
}

#[test]
fn agent_targetless_creation_truth_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_targetless_creation_truth.rs",
        include_str!("local_api_agent_targetless_creation_truth.rs"),
        150,
    )
    .expect("agent targetless creation truth test should stay focused");
}

fn session_body(prompt: &str) -> String {
    format!(
        r#"{{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"{prompt}"}}"#
    )
}

fn assert_no_tool_decision_contains(session: &Value, unexpected: &str) {
    assert_no_timeline_message_contains(session, "tool_decision", unexpected);
}

fn assert_no_timeline_message_contains(session: &Value, kind: &str, unexpected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(!timeline.iter().any(|event| {
        event["kind"] == kind
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains(unexpected))
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
