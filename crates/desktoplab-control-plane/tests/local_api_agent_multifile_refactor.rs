use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn multi_file_refactor_produces_plan_checkpoint_patch_set_diff_review_and_validation() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();

    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{
          "workspaceId":"workspace.workspace",
          "executionBackendId":"backend.ollama",
          "initialPrompt":"refactor shared names",
          "plannedTool":"desktoplab.multi_file_refactor",
          "objective":"rename shared helper",
          "validationCommand":"cargo test -p demo",
          "files":[
            {"path":"a.rs","expected":"fn a() {}","replacement":"fn helper_a() {}"},
            {"path":"b.rs","expected":"fn b() {}","replacement":"fn helper_b() {}"}
          ]
        }"#,
    );

    assert_eq!(session["state"], "blocked");
    assert_eq!(session["summary"], Value::Null);
    assert_timeline_contains(
        &session,
        "Multi-file refactor plan: rename shared helper across 2 files",
    );
    assert_timeline_contains(&session, "state=checkpoint_ready");
    assert_timeline_contains(&session, "Bounded patch set ready: files=2");
    assert_timeline_contains(&session, "Diff review required before approval: a.rs, b.rs");
    assert_timeline_contains(&session, "Validation planned: cargo test -p demo");
    assert_eq!(session["pendingApprovals"].as_array().unwrap().len(), 1);
}

#[test]
fn oversized_multi_file_refactor_blocks_before_checkpoint_or_patch_set() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let files = (0..9)
        .map(|index| format!(r#"{{"path":"file{index}.rs","expected":"old","replacement":"new"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{
              "workspaceId":"workspace.workspace",
              "executionBackendId":"backend.ollama",
              "initialPrompt":"large refactor",
              "plannedTool":"desktoplab.multi_file_refactor",
              "validationCommand":"cargo test",
              "files":[{files}]
            }}"#
        ),
    );

    assert_eq!(session["state"], "blocked");
    assert_timeline_contains(&session, "refactor_patch_set_too_large");
    assert!(!timeline_text(&session).contains("checkpoint_ready"));
}

#[test]
fn agent_multifile_refactor_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_multifile_refactor.rs",
        include_str!("local_api_agent_multifile_refactor.rs"),
        170,
    )
    .expect("agent multifile refactor tests should stay focused");
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
    assert!(output.status.success(), "git command failed");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
