use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn dependency_install_remains_approval_bound_even_when_routine_steps_are_auto_approved() {
    let (_fixture, mut router) = ready_router_with_workspace();
    route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"approve_for_me"}"#,
    );
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Dependency install requires approval.","tool":"desktoplab.run_terminal","arguments":{"command":"npm install left-pad","reason":"install requested dependency"}}"#,
    );

    route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"installa dipendenza"}"#,
    );

    let approvals = route_json(&mut router, "GET", "/v1/approvals", "");
    assert_eq!(approvals["approvals"][0]["action"], "terminal.command");
    assert_eq!(
        approvals["approvals"][0]["operationId"],
        "terminal:npm install left-pad"
    );
}

#[test]
fn lockfile_write_remains_approval_bound_and_generated_artifact_budget_blocks_large_write() {
    let (_fixture, mut router) = ready_router_with_workspace();
    route_json(
        &mut router,
        "POST",
        "/v1/approval-modes/session",
        r#"{"mode":"full_access"}"#,
    );
    router.complete_agent_backend_for_test(
        r#"{"assistantMessage":"Aggiorno il lockfile.","desktoplabAction":{"kind":"create_file","path":"package-lock.json","content":"{}"}}"#,
    );

    route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"aggiorna lockfile"}"#,
    );
    let lockfile_approval_id = approval_id_for(&mut router, "filesystem.write:package-lock.json");
    resolve_approval(&mut router, &lockfile_approval_id, "deny");

    let big_content = "x".repeat(1_048_577);
    router.complete_agent_backend_for_test(format!(
        r#"{{"assistantMessage":"Genero il bundle.","desktoplabAction":{{"kind":"create_file","path":"dist/bundle.js","content":{}}}}}"#,
        serde_json::to_string(&big_content).unwrap()
    ));
    let oversized = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"scrivi dist/bundle.js"}"#,
    );
    let approval_id = approval_id_for(&mut router, "filesystem.write:dist/bundle.js");
    resolve_approval(&mut router, &approval_id, "approve");
    let failed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            oversized["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );
    assert_eq!(failed["state"], "failed");
    assert!(
        failed
            .to_string()
            .contains("generated_artifact_budget_exceeded")
    );
}

#[test]
fn agent_dependency_policy_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_dependency_policy.rs",
        include_str!("local_api_agent_dependency_policy.rs"),
        150,
    )
    .expect("agent dependency policy test should stay focused");
}

fn approval_id_for(router: &mut LocalApiRouter, operation_id: &str) -> String {
    route_json(router, "GET", "/v1/approvals", "")["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|approval| approval["operationId"] == operation_id)
        .and_then(|approval| approval["approvalId"].as_str())
        .unwrap()
        .to_string()
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str, resolution: &str) {
    let body = format!(r#"{{"resolution":"{resolution}"}}"#);
    let _ = route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        &body,
    );
}

fn ready_router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should write");
    assert!(
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&workspace_root)
            .status()
            .expect("git init should run")
            .success()
    );
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
    (fixture, router)
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
