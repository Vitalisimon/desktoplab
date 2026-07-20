use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn followup_read_prompt_targets_the_created_markdown_file() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("DESKTOPLAB_AGENT_NOTES.md"),
        "contenuto reale",
    )
    .expect("notes file should exist");
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Leggo il file creato.","tool":"desktoplab.read_file","arguments":{"path":"DESKTOPLAB_AGENT_NOTES.md"}}"#,
    );

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Leggi il file e dimmi cosa hai scritto"}"#,
    );

    assert_eq!(completed["state"], "completed");
    assert_tool_decision(&completed, "filesystem.read:DESKTOPLAB_AGENT_NOTES.md");
    assert_no_readme_read(&completed);
}

#[test]
fn local_model_file_access_refusal_does_not_invent_a_read_action() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("DESKTOPLAB_AGENT_NOTES.md"),
        "contenuto reale",
    )
    .expect("notes file should exist");
    router.complete_agent_backend_for_test("Mi dispiace, ma non ho accesso al contenuto del file.");

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Leggi il file e dimmi cosa hai scritto"}"#,
    );

    assert_eq!(completed["state"], "completed");
    assert_no_readme_read(&completed);
    assert!(
        !completed
            .to_string()
            .contains("filesystem.read:DESKTOPLAB_AGENT_NOTES.md")
    );
    assert_assistant_message_contains(&completed, "non ho accesso");
}

#[test]
fn edit_prompt_targets_existing_markdown_for_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("DESKTOPLAB_AGENT_NOTES.md"), "bozza")
        .expect("notes file should exist");
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Aggiorno il documento.","desktoplabAction":{"kind":"replace_file","path":"DESKTOPLAB_AGENT_NOTES.md","content":"bozza aggiornata"}}"#,
    );

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"modifica DESKTOPLAB_AGENT_NOTES.md"}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "filesystem.write:DESKTOPLAB_AGENT_NOTES.md"
    );
    assert_no_readme_read(&blocked);
}

#[test]
fn agent_file_creation_truth_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_file_creation_truth.rs",
        include_str!("local_api_agent_file_creation_truth.rs"),
        195,
    )
    .expect("agent file creation truth test should stay focused");
}

fn assert_no_readme_read(session: &Value) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(!timeline.iter().any(|event| {
        event["kind"] == "tool_decision"
            && event["message"]
                .as_str()
                .is_some_and(|message| message.contains("filesystem.read:README.md"))
    }));
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

fn assert_tool_decision(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| {
            event["kind"] == "tool_decision"
                && event["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected))
        }),
        "missing tool decision {expected}: {session}"
    );
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
