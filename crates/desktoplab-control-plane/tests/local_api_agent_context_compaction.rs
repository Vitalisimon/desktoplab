use std::process::Command;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn long_session_compaction_survives_restart_and_keeps_recent_turns() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    create_repo(&workspace);
    let database = fixture.path().join("desktoplab.sqlite");
    let (session_id, workspace_id) = {
        let mut router = ready_router(&database, &workspace);
        router.complete_agent_backend_for_test("assistant turn 0");
        let created = route_json(
            &mut router,
            "POST",
            "/v1/sessions",
            &json!({
                "workspaceId":"workspace.workspace",
                "executionBackendId":"backend.ollama",
                "initialPrompt":"user turn 0"
            })
            .to_string(),
        );
        let session_id = created["sessionId"].as_str().unwrap().to_string();
        let workspace_id = created["workspaceId"].as_str().unwrap().to_string();
        for turn in 1..28 {
            router.complete_agent_backend_for_test(format!("assistant turn {turn}"));
            route_json(
                &mut router,
                "POST",
                &format!("/v1/sessions/{session_id}/messages"),
                &json!({
                    "workspaceId":workspace_id,
                    "executionBackendId":"backend.ollama",
                    "prompt":format!("user turn {turn}")
                })
                .to_string(),
            );
        }
        let context = router
            .workspace_context_for_session_prompt_for_test(&workspace_id, &session_id, "continue")
            .unwrap();
        assert!(context.contains("compaction=desktoplab.extractive.v1"));
        assert!(context.contains("user turn 27"));
        (session_id, workspace_id)
    };

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    let context = restarted
        .workspace_context_for_session_prompt_for_test(&workspace_id, &session_id, "continue")
        .unwrap();
    assert!(context.contains("compaction=desktoplab.extractive.v1"));
    assert!(context.contains("recent_transcript"));
    assert!(context.contains("user turn 27"));
}

#[test]
fn context_compaction_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_context_compaction.rs",
        include_str!("local_api_agent_context_compaction.rs"),
        150,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_compaction.rs",
        include_str!("../src/router/agent_compaction.rs"),
        180,
    )
    .unwrap();
}

fn ready_router(database: &std::path::Path, workspace: &std::path::Path) -> LocalApiRouter {
    let mut router = LocalApiRouter::with_storage_path(database).unwrap();
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
        &xtask::test_http::workspace_open_body(workspace),
    );
    router
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
