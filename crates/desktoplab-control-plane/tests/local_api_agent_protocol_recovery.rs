use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn git_change_wording_completes_without_protocol_recovery_or_repeated_observations() {
    let workspace = TempDir::new().expect("temp workspace");
    create_changed_repo(workspace.path());
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"tool":"desktoplab.git_status","arguments":{}}"#,
        r#"{"tool":"desktoplab.git_diff","arguments":{}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"calculator.js and release-note.md are modified; AGENT_NOTES.md is untracked.","outcome":"changed","evidenceCallIds":["call.1","call.2"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Use Git status and diff, then summarize every changed or untracked file."}}"#
        ),
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        completed["summary"],
        "calculator.js and release-note.md are modified; AGENT_NOTES.md is untracked."
    );
    let timeline = completed["timeline"].to_string();
    assert_eq!(
        timeline.matches("desktoplab.git_status").count(),
        4,
        "{timeline}"
    );
    assert_eq!(
        timeline.matches("desktoplab.git_diff").count(),
        4,
        "{timeline}"
    );
}

#[test]
fn protocol_recovery_integration_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_protocol_recovery.rs",
        include_str!("local_api_agent_protocol_recovery.rs"),
        150,
    )
    .expect("protocol recovery integration test should stay focused");
}

fn create_changed_repo(path: &std::path::Path) {
    run_git(path, &["init", "-b", "main"]);
    std::fs::write(
        path.join("calculator.js"),
        "export const add = (a, b) => a - b;\n",
    )
    .unwrap();
    std::fs::write(
        path.join("release-note.md"),
        "# Release note\n\nStatus: draft\n",
    )
    .unwrap();
    run_git(path, &["add", "."]);
    run_git(
        path,
        &[
            "-c",
            "user.name=DesktopLab Test",
            "-c",
            "user.email=desktoplab@example.invalid",
            "commit",
            "-m",
            "baseline",
        ],
    );
    std::fs::write(
        path.join("calculator.js"),
        "export const add = (a, b) => a + b;\n",
    )
    .unwrap();
    std::fs::write(
        path.join("release-note.md"),
        "# Release note\n\nStatus: ready\n",
    )
    .unwrap();
    std::fs::write(path.join("AGENT_NOTES.md"), "# Agent notes\n").unwrap();
}

fn run_git(path: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(path)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b    5.2 GB"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b    5.2 GB"}"#,
    );
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(workspace),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    serde_json::from_str(response.body()).expect("route response json")
}
