use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn native_product_loop_executes_the_full_workspace_and_git_action_chain() {
    let (_fixture, workspace, remote, mut router) = router_with_workspace();
    router.complete_native_iterative_backend_sequence_for_test([
        call("list-1", "desktoplab.list_files", json!({})),
        call(
            "read-1",
            "desktoplab.read_file",
            json!({"path":"README.md"}),
        ),
        call(
            "search-1",
            "desktoplab.search_text",
            json!({"query":"Demo","path":"."}),
        ),
        call(
            "write-1",
            "desktoplab.write_file",
            json!({"path":"shortcuts.md","content":"# Draft\n"}),
        ),
        call(
            "read-2",
            "desktoplab.read_file",
            json!({"path":"shortcuts.md"}),
        ),
        call(
            "patch-1",
            "desktoplab.patch_file",
            json!({
                "path":"shortcuts.md",
                "expected":"# Draft\n",
                "replacement":"# Keyboard shortcuts\n"
            }),
        ),
        call(
            "terminal-1",
            "desktoplab.run_terminal",
            json!({"command":"git status --short"}),
        ),
        call(
            "tests-1",
            "desktoplab.run_tests",
            json!({"command":"git diff --check"}),
        ),
        call("status-1", "desktoplab.git_status", json!({})),
        call("diff-1", "desktoplab.git_diff", json!({})),
        call(
            "commit-1",
            "desktoplab.commit_changes",
            json!({"message":"docs: add keyboard shortcuts","paths":["shortcuts.md"]}),
        ),
        call(
            "checkpoint-1",
            "desktoplab.create_checkpoint",
            json!({"label":"after-shortcuts"}),
        ),
        call(
            "push-1",
            "desktoplab.push_changes",
            json!({"remote":"origin","branch":"HEAD:refs/heads/main"}),
        ),
        json!({
            "tool":"desktoplab.complete",
            "arguments":{
                "message":"Created, validated, committed, and pushed shortcuts.md.",
                "outcome":"verified",
                "evidenceCallIds":["tests-1"]
            }
        })
        .to_string(),
    ]);

    let mut session = create_session(
        &mut router,
        "Inspect the repo, create and refine shortcuts.md, validate it, commit it, and push it.",
    );
    while session["state"] == "blocked" {
        let approval_id = session["pendingApprovals"][0]["approvalId"]
            .as_str()
            .expect("blocked action should expose its approval")
            .to_string();
        resolve_approval(&mut router, &approval_id);
        session = route_json(&mut router, "GET", "/v1/agent/workspace", "")["session"].clone();
    }

    assert_eq!(session["state"], "completed", "{session}");
    assert_eq!(
        std::fs::read_to_string(workspace.join("shortcuts.md")).unwrap(),
        "# Keyboard shortcuts\n"
    );
    assert_eq!(
        git_stdout(&workspace, &["log", "-1", "--pretty=%s"]).trim(),
        "docs: add keyboard shortcuts"
    );
    assert_eq!(
        git_stdout(&remote, &["rev-parse", "refs/heads/main"]).trim(),
        git_stdout(&workspace, &["rev-parse", "HEAD"]).trim()
    );
    let timeline = session["timeline"].to_string();
    for tool in [
        "list_files",
        "read_file",
        "search_text",
        "write_file",
        "patch_file",
        "run_terminal",
        "run_tests",
        "git_status",
        "git_diff",
        "commit_changes",
        "create_checkpoint",
        "push_changes",
    ] {
        assert!(
            timeline.contains(&format!(
                "state=observed source=agent.iterative canonical=desktoplab.{tool}"
            )),
            "missing real observation for {tool}: {timeline}"
        );
    }
    let trace = session["trace"]["events"].as_array().unwrap();
    assert!(
        trace
            .iter()
            .any(|event| { event["kind"] == "terminal_observed" && event["success"] == true })
    );
    for tool in [
        "write_file",
        "patch_file",
        "run_terminal",
        "run_tests",
        "commit_changes",
        "create_checkpoint",
        "push_changes",
    ] {
        assert_eq!(
            trace
                .iter()
                .filter(|event| {
                    event["source"] == format!("desktoplab.{tool}") && event["mutation"] == true
                })
                .count(),
            1,
            "{tool} should contribute one executed mutation: {trace:?}"
        );
    }
}

#[test]
fn native_action_chain_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_native_action_chain.rs",
        include_str!("local_api_native_action_chain.rs"),
        300,
    )
    .expect("native action chain should stay focused");
}

fn call(id: &str, tool: &str, arguments: Value) -> String {
    json!({"id":id,"tool":tool,"arguments":arguments}).to_string()
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    let workspace_id =
        route_json(router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"].clone();
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":prompt
        })
        .to_string(),
    )
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}

fn router_with_workspace() -> (
    TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    LocalApiRouter,
) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace = fixture.path().join("workspace");
    let remote = fixture.path().join("remote.git");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&remote).unwrap();
    run_git(&workspace, &["init", "-b", "main"]);
    run_git(
        &workspace,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace, &["config", "user.name", "DesktopLab Test"]);
    run_git(&remote, &["init", "--bare"]);
    std::fs::write(workspace.join("README.md"), "# Demo\n").unwrap();
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    run_git(
        &workspace,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (fixture, workspace, remote, router)
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
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(root: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
