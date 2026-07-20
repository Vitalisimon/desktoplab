use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workspace_open_and_git_operations_use_real_repository_status_and_diff() {
    let fixture = dirty_repo();
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let workspace = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&fixture.path()),
    );

    assert_eq!(workspace["apiState"], "dirty");
    assert!(
        workspace["statusEntries"][0]
            .as_str()
            .unwrap()
            .contains("README.md")
    );
    assert!(
        workspace["diffText"]
            .as_str()
            .unwrap()
            .contains("changed by product proof")
    );
    assert_eq!(workspace["checkpointStatus"], "ready");
    assert_eq!(workspace["canCheckpointRiskyExecution"], true);

    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    assert_eq!(operations["workspaceState"], "dirty");
    assert!(
        operations["statusEntries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry.as_str().unwrap().contains("README.md"))
    );
    assert!(
        operations["changedFiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file == "candidate proof.md")
    );
    assert!(
        operations["diffPreview"]
            .as_str()
            .unwrap()
            .contains("changed by product proof")
    );
    assert_eq!(operations["commit"]["supported"], true);
    assert_eq!(operations["commit"]["requiresApproval"], true);
    assert_eq!(operations["push"]["requiresApproval"], true);
}

#[test]
fn git_commit_and_push_routes_require_matching_approval_records() {
    let mut router = LocalApiRouter::default();

    let commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        r#"{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"agent change"}"#,
    );
    assert_eq!(commit["status"], "blocked");
    assert_eq!(commit["reason"], "approval_required");

    let push = route_json(
        &mut router,
        "POST",
        "/v1/git/push",
        r#"{"workspaceId":"workspace.desktoplab","remote":"origin","branch":"main"}"#,
    );
    assert_eq!(push["status"], "blocked");
    assert_eq!(push["reason"], "approval_required");
}

#[test]
fn git_commit_route_creates_real_commit_after_matching_approval() {
    let fixture = dirty_repo();
    configure_git_identity(fixture.path());
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&fixture.path()),
    );
    let operations = route_json(&mut router, "GET", "/v1/git/operations", "");
    let change_fingerprint = operations["commit"]["changeFingerprint"]
        .as_str()
        .expect("git operations should expose a diff fingerprint");
    let changed_files = serde_json::to_string(&operations["changedFiles"]).unwrap();
    let approval = route_json(
        &mut router,
        "POST",
        "/v1/approvals",
        &format!(
            r#"{{"sessionId":"session.1","action":"git.commit","operationId":"git.commit","payload":{{"message":"agent change","sessionId":"session.1","changeFingerprint":"{change_fingerprint}","changedFiles":{changed_files}}}}}"#
        ),
    );
    route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/approvals/{}/resolve",
            approval["approvalId"].as_str().unwrap()
        ),
        r#"{"resolution":"approve"}"#,
    );

    let commit = route_json(
        &mut router,
        "POST",
        "/v1/git/commit",
        &format!(
            r#"{{"workspaceId":"workspace.desktoplab","sessionId":"session.1","message":"agent change","changeFingerprint":"{change_fingerprint}","changedFiles":{changed_files},"approvalId":"{}"}}"#,
            approval["approvalId"].as_str().unwrap(),
        ),
    );

    assert_eq!(commit["status"], "committed");
    assert_ne!(commit["commitHash"], "local-preview");
    let head = git_stdout(fixture.path(), &["rev-parse", "HEAD"]);
    assert_eq!(commit["commitHash"].as_str().unwrap(), head.trim());
    assert!(git_stdout(fixture.path(), &["log", "-1", "--pretty=%B"]).contains("agent change"));
}

#[test]
fn git_diff_product_truth_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/git_diff_product_truth.rs",
        include_str!("git_diff_product_truth.rs"),
        210,
    )
    .expect("git diff product truth test should stay focused");
}

fn dirty_repo() -> TempDir {
    let repo = TempDir::new().unwrap();
    run_git(repo.path(), &["init"]);
    std::fs::write(repo.path().join("README.md"), "changed by product proof\n").unwrap();
    std::fs::write(repo.path().join("candidate proof.md"), "path with spaces\n").unwrap();
    repo
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

fn run_git(cwd: &std::path::Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap()
            .success()
    );
}

fn configure_git_identity(cwd: &std::path::Path) {
    run_git(cwd, &["config", "user.email", "desktoplab@example.local"]);
    run_git(cwd, &["config", "user.name", "DesktopLab Test"]);
}

fn git_stdout(cwd: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn route_json(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> serde_json::Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
