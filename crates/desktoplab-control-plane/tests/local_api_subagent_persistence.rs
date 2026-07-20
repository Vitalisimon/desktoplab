use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn parent_child_relation_survives_control_plane_restart() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::with_storage_path(&database).unwrap();
    mark_setup_ready(&mut router);
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    let workspace_id = opened["workspaceId"].as_str().unwrap();
    router.complete_agent_backend_for_test("Parent ready.");
    let parent = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"parent"}}"#
        ),
    );
    let parent_id = parent["sessionId"].as_str().unwrap();
    router.complete_agent_backend_for_test("Child complete.");
    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(r#"{{"parentSessionId":"{parent_id}","prompt":"inspect","intent":"read_only"}}"#),
    );
    let child_id = child["subagentId"].as_str().unwrap().to_string();
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    let restored = route_json(
        &mut restarted,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    );

    assert_eq!(restored["parentSessionId"], parent_id);
    assert_eq!(restored["state"], "completed");
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
        .unwrap();
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

#[test]
fn subagent_persistence_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_subagent_persistence.rs",
        include_str!("local_api_subagent_persistence.rs"),
        125,
    )
    .expect("subagent persistence test should stay focused");
}
