use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn iterative_agent_executes_approved_directory_move_and_delete_actions() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    std::fs::write(workspace.path().join("draft.md"), "draft\n").unwrap();
    let (mut router, workspace_id) = ready_router(workspace.path());
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"mkdir-1","tool":"desktoplab.create_directory","arguments":{"path":"docs/archive"}}"#,
        r#"{"id":"move-1","tool":"desktoplab.move_path","arguments":{"source":"draft.md","destination":"docs/draft.md"}}"#,
        r#"{"id":"delete-1","tool":"desktoplab.delete_path","arguments":{"path":"docs/archive","recursive":false}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Reorganized the workspace.","outcome":"changed","evidenceCallIds":["mkdir-1","move-1","delete-1"]}}"#,
    ]);

    let mut state = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"Reorganize draft.md under docs."}}"#
        ),
    );
    for _ in 0..3 {
        assert_eq!(state["state"], "blocked", "{state}");
        let approval_id = state["pendingApprovals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
        route_json(
            &mut router,
            "POST",
            &format!("/v1/approvals/{approval_id}/resolve"),
            r#"{"resolution":"approve"}"#,
        );
        state = route_json(&mut router, "GET", "/v1/agent/workspace", "")["session"].clone();
    }

    assert_eq!(state["state"], "completed", "{state}");
    assert_eq!(state["summary"], "Reorganized the workspace.");
    assert!(!workspace.path().join("draft.md").exists());
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("docs/draft.md")).unwrap(),
        "draft\n"
    );
    assert!(!workspace.path().join("docs/archive").exists());
}

#[test]
fn filesystem_mutation_product_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_filesystem_mutations.rs",
        include_str!("local_api_agent_filesystem_mutations.rs"),
        150,
    )
    .unwrap();
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
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
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

fn create_repo(path: &std::path::Path) {
    assert!(
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
}
