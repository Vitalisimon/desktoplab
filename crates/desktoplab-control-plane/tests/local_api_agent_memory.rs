use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn workspace_memory_persists_safe_repo_summary_without_secrets_or_raw_provider_payloads() {
    let temp = TempDir::new().expect("temp dir should exist");
    let db = temp.path().join("desktoplab.sqlite");
    let workspace = temp.path().join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should exist");
    std::fs::write(workspace.join("README.md"), "# DesktopLab\n").unwrap();
    let git_init = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&workspace)
        .output()
        .expect("git init should run");
    assert!(git_init.status.success());

    {
        let mut router = LocalApiRouter::with_storage_path(&db).expect("router should open");
        mark_setup_ready(&mut router);
        route_json(
            &mut router,
            "POST",
            "/v1/workspaces/open",
            &xtask::test_http::workspace_open_body(&workspace),
        );
        let saved = route_json(
            &mut router,
            "POST",
            "/v1/workspaces/workspace.desktoplab/memory",
            r#"{
              "kind":"repo_summary",
              "title":"Repository summary",
              "summary":"Rust control plane with api_key=sk-live-secret omitted",
              "decisions":["Use local-first runtime","token=ghp_secret must not persist"],
              "source":"agent.summary",
              "rawProviderPayload":{"choices":[{"message":"do not store provider envelope"}]}
            }"#,
        );
        assert_eq!(
            saved["memories"][0]["summary"],
            "Rust control plane with [REDACTED] omitted"
        );
        assert_eq!(
            saved["memories"][0]["decisions"][1],
            "[REDACTED] must not persist"
        );
        assert!(saved["memories"][0].get("rawProviderPayload").is_none());
        let context = router
            .workspace_context_for_prompt_for_test("workspace.desktoplab", "continue the work", &[])
            .expect("workspace context should exist");
        assert!(context.contains("Rust control plane with [REDACTED] omitted"));
        assert!(context.contains("Use local-first runtime"));
        assert!(!context.contains("sk-live-secret"));
        assert!(!context.contains("ghp_secret"));
    }

    let mut reopened = LocalApiRouter::with_storage_path(&db).expect("router should reopen");
    let memories = route_json(
        &mut reopened,
        "GET",
        "/v1/workspaces/workspace.desktoplab/memory",
        "",
    );
    assert_eq!(memories["workspaceId"], "workspace.desktoplab");
    assert_eq!(memories["memories"].as_array().unwrap().len(), 1);
    let serialized = memories.to_string();
    assert!(!serialized.contains("sk-live-secret"));
    assert!(!serialized.contains("rawProviderPayload"));
    assert!(!serialized.contains("provider envelope"));
    let context = reopened
        .workspace_context_for_prompt_for_test(
            "workspace.desktoplab",
            "continue after restart",
            &[],
        )
        .expect("reopened context should exist");
    assert!(context.contains("Rust control plane with [REDACTED] omitted"));
    assert!(!context.contains("sk-live-secret"));
}

#[test]
fn workspace_memory_delete_removes_only_the_target_entry() {
    let temp = TempDir::new().expect("temp dir should exist");
    let workspace = temp.path().join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should exist");
    let git_init = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&workspace)
        .output()
        .expect("git init should run");
    assert!(git_init.status.success());
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.desktoplab/memory",
        r#"{"title":"One","summary":"First"}"#,
    );
    let saved = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/workspace.desktoplab/memory",
        r#"{"title":"Two","summary":"Second"}"#,
    );
    let memory_id = saved["memories"][0]["memoryId"].as_str().unwrap();

    route_json(
        &mut router,
        "POST",
        &format!("/v1/workspaces/memory/{memory_id}/delete"),
        "",
    );

    let memories = route_json(
        &mut router,
        "GET",
        "/v1/workspaces/workspace.desktoplab/memory",
        "",
    );
    assert_eq!(memories["memories"].as_array().unwrap().len(), 1);
    assert_eq!(memories["memories"][0]["title"], "Two");
}

#[test]
fn agent_memory_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_memory.rs",
        include_str!("local_api_agent_memory.rs"),
        180,
    )
    .expect("agent memory tests should stay focused");
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
