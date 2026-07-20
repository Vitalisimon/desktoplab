use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn transcript_hides_structured_json_and_keeps_tool_details() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_native_iterative_backend_sequence_for_test([
        r##"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"note.md","content":"# Note\n"}}"##,
    ]);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"create note.md"}"#,
    );

    assert_eq!(blocked["state"], "blocked");
    assert_transcript_contains(&blocked, "planned · desktoplab.write_file");
    assert_transcript_excludes(&blocked, "\"arguments\"");
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "filesystem.write:note.md"
    );
}

#[test]
fn completed_transcript_collapses_tool_progress_but_keeps_details() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Read README.md.","outcome":"answered","evidenceCallIds":["read-1"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"read README.md"}"#,
    );

    assert_eq!(completed["state"], "completed");
    assert_transcript_excludes(&completed, "filesystem.read:README.md");
    assert_timeline_contains(&completed, "Read README.md");
}

#[test]
fn canonical_tool_call_hides_provider_json_from_visible_turns() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_native_iterative_backend_sequence_for_test([
        r##"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"##,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Read README.md.","outcome":"answered","evidenceCallIds":["read-1"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"read README.md"}"#,
    );

    assert_eq!(completed["state"], "completed");
    assert_transcript_excludes(&completed, "read_file");
    assert_transcript_excludes(&completed, "\"arguments\"");
    assert_timeline_contains(&completed, "Read README.md");
}

#[test]
fn sequential_canonical_tool_calls_hide_provider_json() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::create_dir_all(workspace_root.join("src")).unwrap();
    std::fs::write(workspace_root.join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        r##"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"##,
        r##"{"id":"read-2","tool":"desktoplab.read_file","arguments":{"path":"src/lib.rs"}}"##,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Read README.md and src/lib.rs.","outcome":"answered","evidenceCallIds":["read-1","read-2"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"read README.md and src/lib.rs"}"#,
    );

    assert_eq!(completed["state"], "completed");
    assert_transcript_excludes(&completed, "read_file");
    assert_transcript_excludes(&completed, "\"arguments\"");
}

#[test]
fn agent_transcript_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_transcript.rs",
        include_str!("local_api_agent_transcript.rs"),
        200,
    )
    .expect("agent transcript test should stay focused");
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = serde_json::to_string(&session["timeline"]).unwrap();
    assert!(
        timeline.contains(expected),
        "missing {expected}: {timeline}"
    );
}

fn assert_transcript_contains(session: &Value, expected: &str) {
    let transcript = serde_json::to_string(&session["transcript"]).unwrap();
    assert!(
        transcript.contains(expected),
        "missing {expected}: {transcript}"
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
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").expect("README should write");
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
