use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn rollback_preview_and_approved_restore_keep_untracked_files() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Demo\nchanged\n").unwrap();
    std::fs::write(workspace_root.join("scratch.md"), "local only\n").unwrap();

    let preview = route_json(
        &mut router,
        "POST",
        "/v1/git/savepoints/HEAD/rollback/preview",
        "",
    );
    assert_eq!(preview["status"], "preview");
    assert!(
        preview["changedFiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file == "README.md")
    );
    assert!(
        preview["protectedUntrackedFiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file == "scratch.md")
    );

    let approval = route_json(
        &mut router,
        "POST",
        "/v1/approvals",
        r#"{"sessionId":"session.local","action":"git.rollback","operationId":"HEAD"}"#,
    );
    let approval_id = approval["approvalId"].as_str().unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let restored = route_json(
        &mut router,
        "POST",
        "/v1/git/savepoints/HEAD/rollback",
        &format!(r#"{{"approvalId":"{approval_id}"}}"#),
    );

    assert_eq!(restored["status"], "restored");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("README.md")).unwrap(),
        "# Demo\n"
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("scratch.md")).unwrap(),
        "local only\n"
    );
}

#[test]
fn git_operations_lists_persisted_savepoints() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    desktoplab_workspace::SavePointManager::default()
        .create(&workspace_root, "session.1")
        .unwrap();

    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");

    assert_eq!(operations["savePoints"][0]["sessionId"], "session.1");
    assert_eq!(
        operations["savePoints"][0]["savePointId"],
        "desktoplab/savepoints/session.1"
    );
    assert_eq!(operations["savePoints"][0]["rollbackSupported"], true);
}

#[test]
fn agent_rollback_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_rollback.rs",
        include_str!("local_api_agent_rollback.rs"),
        165,
    )
    .expect("agent rollback route tests should stay focused");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    run_git(&workspace_root, &["init", "-b", "main"]);
    std::fs::write(workspace_root.join("README.md"), "# Demo\n").expect("README should write");
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
        .args([
            "-c",
            "user.name=DesktopLab",
            "-c",
            "user.email=desktoplab@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "{}",
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
