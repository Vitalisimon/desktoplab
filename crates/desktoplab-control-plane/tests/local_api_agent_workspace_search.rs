use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn agent_workspace_context_searches_relevant_files_without_secrets() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::create_dir_all(workspace_root.join("apps/desktop/src/features/productization"))
        .unwrap();
    std::fs::write(workspace_root.join("README.md"), "# DesktopLab\n").unwrap();
    std::fs::write(
        workspace_root.join(".env"),
        "OPENAI_API_KEY=sk-secret\ncomposer",
    )
    .unwrap();
    std::fs::write(
        workspace_root.join("apps/desktop/src/features/productization/AgentComposer.tsx"),
        "export function AgentComposer() { return 'composer'; }\n",
    )
    .unwrap();

    let context = router
        .workspace_context_for_prompt_for_test("workspace.desktoplab", "trova composer", &[])
        .expect("context should build");

    assert!(context.contains("AgentComposer.tsx"), "{context}");
    assert!(
        context.contains("context_reason=RetrievedEvidence"),
        "{context}"
    );
    assert!(context.contains("files=README.md"), "{context}");
    assert!(
        context.contains("export function AgentComposer"),
        "{context}"
    );
    assert!(!context.contains("sk-secret"), "{context}");
}

#[test]
fn approved_write_records_incremental_context_index_update() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    let workspace_id =
        route_json(&mut router, "GET", "/v1/agent/workspace", "")["context"]["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo indexed.md.","desktoplabAction":{"kind":"create_file","path":"indexed.md","content":"# Indexed\n"}}"##,
    );
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","initialPrompt":"crea indexed.md"}}"#
        ),
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );

    let completed = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            blocked["sessionId"].as_str().unwrap()
        ),
        &format!(
            r#"{{"workspaceId":"{workspace_id}","executionBackendId":"backend.ollama","prompt":"continue","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_timeline_contains(
        &completed,
        "context_index_updated path=indexed.md mode=incremental",
    );
}

#[test]
fn local_api_agent_workspace_search_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_workspace_search.rs",
        include_str!("local_api_agent_workspace_search.rs"),
        190,
    )
    .expect("workspace search control-plane test should stay focused");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().unwrap();
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir_all(&workspace_root).unwrap();
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

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["message"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        timeline.contains(expected),
        "timeline should contain {expected}: {timeline}"
    );
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
