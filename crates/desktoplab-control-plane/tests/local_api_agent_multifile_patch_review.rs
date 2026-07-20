use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn multi_file_refactor_requires_one_patch_set_approval_then_applies_all_files() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    write_file(&workspace_root, "a.rs", "fn a() {}\n");
    write_file(&workspace_root, "b.rs", "fn b() {}\n");
    commit_fixture_files(&workspace_root);

    let session = create_patch_set_session(&mut router, "fn a() {}", "fn b() {}");

    assert_eq!(session["state"], "blocked");
    assert_timeline_contains(
        &session,
        "Multi-file refactor plan: rename helpers across 2 files",
    );
    assert_timeline_contains(&session, "state=checkpoint_ready");
    assert_timeline_contains(&session, "Bounded patch set ready: files=2");
    assert_timeline_contains(&session, "Diff review required before approval: a.rs, b.rs");
    let approval = only_pending_approval(&session);
    assert_eq!(approval["action"], "filesystem.write");
    assert_eq!(
        approval["operationId"],
        "filesystem.patch:multi-file patch set"
    );
    assert_eq!(approval["title"], "Patch multi-file patch set");

    let approved = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/approvals/{}/resolve",
            approval["approvalId"].as_str().unwrap()
        ),
        r#"{"resolution":"approve"}"#,
    );
    assert_eq!(approved["state"], "approved");

    let continued = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            session["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{}"}}"#,
            approval["approvalId"].as_str().unwrap()
        ),
    );

    assert_eq!(continued["state"], "completed");
    assert_eq!(read_file(&workspace_root, "a.rs"), "fn helper_a() {}\n");
    assert_eq!(read_file(&workspace_root, "b.rs"), "fn helper_b() {}\n");
    assert_timeline_contains(&continued, "Multi-file patch applied: a.rs, b.rs");
    assert_timeline_contains(&continued, "a.rs expected_bytes=9 replacement_bytes=16");
    assert_timeline_contains(&continued, "b.rs expected_bytes=9 replacement_bytes=16");
}

#[test]
fn multi_file_refactor_blocks_without_partial_write_when_later_file_conflicts() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    write_file(&workspace_root, "a.rs", "fn a() {}\n");
    write_file(&workspace_root, "b.rs", "fn already_changed() {}\n");
    commit_fixture_files(&workspace_root);

    let session = create_patch_set_session(&mut router, "fn a() {}", "fn b() {}");
    let approval = only_pending_approval(&session);
    route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/approvals/{}/resolve",
            approval["approvalId"].as_str().unwrap()
        ),
        r#"{"resolution":"approve"}"#,
    );

    let continued = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            session["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{}"}}"#,
            approval["approvalId"].as_str().unwrap()
        ),
    );

    assert_eq!(continued["state"], "failed");
    assert_eq!(read_file(&workspace_root, "a.rs"), "fn a() {}\n");
    assert_eq!(
        read_file(&workspace_root, "b.rs"),
        "fn already_changed() {}\n"
    );
    assert_timeline_contains(&continued, "multi_file_patch_expected_content_missing:b.rs");
}

#[test]
fn agent_multifile_patch_review_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_multifile_patch_review.rs",
        include_str!("local_api_agent_multifile_patch_review.rs"),
        220,
    )
    .expect("agent multifile patch review tests should stay focused");
}

fn create_patch_set_session(
    router: &mut LocalApiRouter,
    a_expected: &str,
    b_expected: &str,
) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{
              "workspaceId":"workspace.workspace",
              "executionBackendId":"backend.ollama",
              "initialPrompt":"refactor shared names",
              "plannedTool":"desktoplab.multi_file_refactor",
              "objective":"rename helpers",
              "validationCommand":"cargo test -p demo",
              "files":[
                {{"path":"a.rs","expected":"{a_expected}","replacement":"fn helper_a() {{}}"}},
                {{"path":"b.rs","expected":"{b_expected}","replacement":"fn helper_b() {{}}"}}
              ]
            }}"#
        ),
    )
}

fn only_pending_approval(session: &Value) -> &Value {
    let approvals = session["pendingApprovals"].as_array().unwrap();
    assert_eq!(
        approvals.len(),
        1,
        "expected one pending approval: {session:#?}"
    );
    &approvals[0]
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    assert!(
        timeline_text(session).contains(expected),
        "timeline should contain {expected}: {session:#?}"
    );
}

fn timeline_text(session: &Value) -> String {
    session["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["message"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    router.complete_agent_backend_for_test("Refactor completed with executor evidence.");
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

fn write_file(root: &std::path::Path, path: &str, contents: &str) {
    std::fs::write(root.join(path), contents).expect("fixture file should write");
}

fn read_file(root: &std::path::Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).expect("fixture file should read")
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
        .expect("git command should run");
    assert!(output.status.success(), "git command failed");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
