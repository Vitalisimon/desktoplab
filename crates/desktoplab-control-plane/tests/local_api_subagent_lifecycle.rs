use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn child_session_has_parent_identity_and_can_be_waited_and_closed() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_agent_backend_for_test("Delegated inspection completed.");

    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(
            r#"{{"parentSessionId":"{parent_id}","prompt":"inspect README","intent":"read_only"}}"#
        ),
    );
    let child_id = child["subagentId"].as_str().unwrap();
    let waited = route_json(
        &mut router,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    );
    let closed = route_json(
        &mut router,
        "POST",
        &format!("/v1/agent/subagents/{child_id}/close"),
        "{}",
    );

    assert_eq!(waited["parentSessionId"], parent_id);
    assert_eq!(waited["state"], "completed");
    assert_eq!(closed["closed"], true);
}

#[test]
fn write_capable_child_is_queued_with_a_real_isolated_worktree() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());

    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(
            r#"{{"parentSessionId":"{parent_id}","prompt":"prepare an isolated change","intent":"write_capable"}}"#
        ),
    );

    assert_eq!(child["state"], "running");
    assert!(
        std::path::Path::new(child["worktree"].as_str().unwrap()).is_dir(),
        "{child}"
    );
}

#[test]
fn running_child_can_be_cancelled_but_not_closed_first() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());
    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(
            r#"{{"parentSessionId":"{parent_id}","prompt":"wait for cancellation","intent":"read_only"}}"#
        ),
    );
    let child_id = child["subagentId"].as_str().unwrap();

    let premature = route(
        &mut router,
        "POST",
        &format!("/v1/agent/subagents/{child_id}/close"),
        "{}",
    );
    assert_eq!(premature.status(), "400 Bad Request");
    let cancelled = route_json(
        &mut router,
        "POST",
        &format!("/v1/agent/subagents/{child_id}/cancel"),
        "{}",
    );
    assert_eq!(cancelled["state"], "cancelled");
    let closed = route_json(
        &mut router,
        "POST",
        &format!("/v1/agent/subagents/{child_id}/close"),
        "{}",
    );
    assert_eq!(closed["closed"], true);
}

#[test]
fn model_spawn_tool_creates_a_real_child_without_replacing_the_parent() {
    let (_fixture, mut router) = router_with_workspace();
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"spawn-1","tool":"desktoplab.spawn_subagent","arguments":{"prompt":"inspect README","intent":"read_only"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Delegated repository inspection.","outcome":"executed","evidenceCallIds":["spawn-1"]}}"#,
    ]);

    let parent = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"delegate inspection"}"#,
    );
    let parent_id = parent["sessionId"].as_str().unwrap();
    let sessions = route_json(&mut router, "GET", "/v1/sessions", "");
    let child_id = sessions["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|session| {
            (session["sessionId"] != parent_id)
                .then(|| session["sessionId"].as_str().unwrap().to_string())
        })
        .expect("the tool must create a real child session");
    let child = route_json(
        &mut router,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    );
    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(parent["state"], "completed", "{parent}");
    assert_eq!(child["parentSessionId"], parent_id);
    assert_eq!(child["state"], "running");
    assert_eq!(workspace["session"]["sessionId"], parent_id);
}

#[test]
fn model_can_message_observe_cancel_and_close_only_its_real_child() {
    let (_fixture, mut router) = router_with_workspace();
    let parent_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());
    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(
            r#"{{"parentSessionId":"{parent_id}","prompt":"inspect README","intent":"read_only"}}"#
        ),
    );
    let child_id = child["subagentId"].as_str().unwrap();
    router.complete_native_iterative_backend_sequence_for_test([
        format!(r#"{{"id":"send-1","tool":"desktoplab.send_subagent","arguments":{{"subagentId":"{child_id}","prompt":"also inspect Cargo.toml"}}}}"#),
        format!(r#"{{"id":"get-1","tool":"desktoplab.get_subagent","arguments":{{"subagentId":"{child_id}"}}}}"#),
        format!(r#"{{"id":"cancel-1","tool":"desktoplab.cancel_subagent","arguments":{{"subagentId":"{child_id}"}}}}"#),
        format!(r#"{{"id":"close-1","tool":"desktoplab.close_subagent","arguments":{{"subagentId":"{child_id}"}}}}"#),
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Child lifecycle completed.","outcome":"executed","evidenceCallIds":["send-1","get-1","cancel-1","close-1"]}}"#.to_string(),
    ]);

    let parent = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{parent_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"manage the child"}"#,
    );
    let closed = route_json(
        &mut router,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    );

    assert_eq!(parent["state"], "completed", "{parent}");
    assert_eq!(closed["state"], "cancelled");
    assert_eq!(closed["closed"], true);
}

#[test]
fn model_cannot_control_a_child_owned_by_another_parent() {
    let (_fixture, mut router) = router_with_workspace();
    let owner_id = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test(Vec::<String>::new());
    let child = route_json(
        &mut router,
        "POST",
        "/v1/agent/subagents",
        &format!(r#"{{"parentSessionId":"{owner_id}","prompt":"inspect","intent":"read_only"}}"#),
    );
    let child_id = child["subagentId"].as_str().unwrap();
    router.complete_agent_backend_for_test("Second parent ready.");
    let other_parent = create_parent(&mut router);
    router.complete_native_iterative_backend_sequence_for_test([
        format!(r#"{{"id":"foreign-1","tool":"desktoplab.get_subagent","arguments":{{"subagentId":"{child_id}"}}}}"#),
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Observed child.","outcome":"answered","evidenceCallIds":[]}}"#.to_string(),
    ]);

    let rejected = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{other_parent}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"inspect foreign child"}"#,
    );
    let unchanged = route_json(
        &mut router,
        "GET",
        &format!("/v1/agent/subagents/{child_id}"),
        "",
    );

    assert_eq!(rejected["state"], "failed", "{rejected}");
    assert_eq!(unchanged["parentSessionId"], owner_id);
    assert_eq!(unchanged["state"], "running");
}

fn create_parent(router: &mut LocalApiRouter) -> String {
    router.complete_agent_backend_for_test("Parent ready.");
    route_json(
        router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":"prepare parent"}"#,
    )["sessionId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn router_with_workspace() -> (TempDir, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (fixture, router)
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
        .args(["-c", "user.name=DesktopLab", "-c", "user.email=x@y.z"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = route(router, method, path, body);
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

fn route(
    router: &mut LocalApiRouter,
    method: &str,
    path: &str,
    body: &str,
) -> desktoplab_control_plane::ApiRouteResponse {
    router.route(method, path, body).unwrap()
}

#[test]
fn subagent_lifecycle_test_stays_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-control-plane/tests/local_api_subagent_lifecycle.rs",
            include_str!("local_api_subagent_lifecycle.rs"),
            340,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_subagent_tools.rs",
            include_str!("../src/router/agent_subagent_tools.rs"),
            220,
        ),
        (
            "crates/desktoplab-control-plane/src/router/agent_model_tools.rs",
            include_str!("../src/router/agent_model_tools.rs"),
            140,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, limit)
            .expect("subagent lifecycle sources should stay focused");
    }
}
