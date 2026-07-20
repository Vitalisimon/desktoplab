use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn fenced_and_concatenated_provider_tool_json_recovers_without_raw_transcript() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "Important module.\n").unwrap();
    router.complete_agent_backend_for_test(
        "```json\n{\"assistantMessage\":\"Leggo README.\",\"tool\":\"desktoplab.read_file\",\"arguments\":{\"path\":\"README.md\"}}\n```",
    );
    let fenced = create_session(&mut router, "leggi readme");
    assert_eq!(fenced["state"], "completed");
    assert_timeline_contains(&fenced, "provider_output_recovery:fenced_json");
    assert_transcript_excludes(&fenced, "\"tool\":\"desktoplab.read_file\"");

    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Leggo ancora.","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}{"assistantMessage":"duplicato","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
    );
    let concatenated = create_session(&mut router, "leggi ancora");
    assert_eq!(concatenated["state"], "completed");
    assert_timeline_contains(&concatenated, "provider_output_recovery:concatenated_json");
    assert_transcript_excludes(&concatenated, r#""arguments""#);
}

#[test]
fn canonical_name_envelope_from_ollama_content_executes_real_tool() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "Canonical live module.\n").unwrap();
    router.complete_agent_backend_for_test(
        "```json\n{\"name\":\"desktoplab.read_file\",\"arguments\":{\"path\":\"README.md\"}}\n```",
    );

    let completed = create_session(&mut router, "leggi readme");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(&completed, "Canonical live module.");
    assert_timeline_contains(&completed, "provider_output_recovery:fenced_json");
    assert_transcript_excludes(&completed, "desktoplab.read_file");
}

#[test]
fn canonical_name_envelope_after_model_prose_executes_real_tool() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "Mixed prose module.\n").unwrap();
    router.complete_agent_backend_for_test(
        "I will inspect the file.\n```json\n{\"name\":\"desktoplab.read_file\",\"arguments\":{\"path\":\"README.md\"}}\n```",
    );

    let completed = create_session(&mut router, "leggi readme");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(&completed, "Mixed prose module.");
    assert_timeline_contains(&completed, "provider_output_recovery:mixed_prose_json");
    assert_transcript_excludes(&completed, "desktoplab.read_file");
}

#[test]
fn structured_completion_finishes_without_approval_or_raw_envelope() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"","tool":"desktoplab.complete","arguments":{"message":"The repository inspection is complete."}}"#,
    );

    let completed = create_session(&mut router, "summarize the inspected repository");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "The repository inspection is complete.");
    assert_transcript_excludes(&completed, "desktoplab.complete");
}

#[test]
fn structured_completion_is_valid_for_a_file_scoped_read_goal() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"calculator.js returns the observed result."}}"#,
    );

    let completed = create_session(
        &mut router,
        "Read calculator.js and tell me exactly what the add function returns.",
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "calculator.js returns the observed result.");
}

#[test]
fn malformed_unsafe_provider_action_fails_without_raw_envelope_as_prose() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Creo file","desktoplabAction":{"kind":"create_file","path":"unsafe.md","content":"unterminated""#,
    );

    let failed = create_session(&mut router, "crea unsafe.md");

    assert_eq!(failed["state"], "failed");
    assert_timeline_contains(&failed, "provider_output_recovery:invalid_json");
    assert_timeline_contains(&failed, "provider_output_recovery:unrecognized_shape");
    assert_timeline_contains(&failed, "argumentsKind");
    assert_timeline_contains(&failed, "topLevelKeys");
    assert_transcript_excludes(&failed, "unterminated");
    assert_eq!(failed["failureClassification"]["primary"], "tool_misuse");
    assert_eq!(
        failed["failureClassification"]["userMessage"],
        "The model returned an invalid tool request. DesktopLab stopped without applying it."
    );
}

#[test]
fn followup_while_pending_reuses_existing_approval_without_provider_reexecution() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo note.","desktoplabAction":{"kind":"create_file","path":"note.md","content":"# Note\n"}}"##,
    );
    let first = create_session(&mut router, "crea note.md");
    assert_eq!(first["pendingApprovals"].as_array().unwrap().len(), 1);

    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo note.","desktoplabAction":{"kind":"create_file","path":"note.md","content":"# Note\n"}}"##,
    );
    let second = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            first["sessionId"].as_str().unwrap()
        ),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"crea note.md"}"#,
    );
    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");

    assert_eq!(second["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_eq!(approvals["approvals"].as_array().unwrap().len(), 1);
    assert_timeline_contains(&second, "session continuity pending user choice");
}

#[test]
fn local_api_agent_provider_recovery_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_provider_recovery.rs",
        include_str!("local_api_agent_provider_recovery.rs"),
        210,
    )
    .expect("agent provider recovery tests should stay focused");
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

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = serde_json::to_string(&session["timeline"]).unwrap();
    assert!(
        timeline.contains(expected),
        "missing {expected}: {timeline}"
    );
}

fn assert_transcript_excludes(session: &Value, unexpected: &str) {
    let transcript = serde_json::to_string(&session["transcript"]).unwrap();
    assert!(
        !transcript.contains(unexpected),
        "unexpected {unexpected}: {transcript}"
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}
