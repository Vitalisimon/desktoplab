use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use std::path::Path;

#[test]
fn ready_agent_route_does_not_fabricate_a_workspace() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let before = route_json(&mut router, "GET", "/v1/workspaces", "");
    assert!(before["workspaces"].as_array().unwrap().is_empty());

    let agent = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert_eq!(agent["route"]["status"], "blocked");
    assert_eq!(
        agent["route"]["blockedReasons"][0],
        "workspace_not_selected"
    );
    assert_eq!(agent["context"], Value::Null);
    assert_eq!(agent["session"], Value::Null);

    let after = route_json(&mut router, "GET", "/v1/workspaces", "");
    assert!(after["workspaces"].as_array().unwrap().is_empty());
}

#[test]
fn session_creation_cannot_target_a_synthetic_default_workspace() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"Inspect the repository"}"#,
    );
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["blockedReason"], "workspace_not_selected");
    assert_eq!(blocked["sessionId"], "session.blocked");

    let sessions = route_json(
        &mut router,
        "GET",
        "/v1/sessions?workspace_id=workspace.desktoplab",
        "",
    );
    assert!(sessions["sessions"].as_array().unwrap().is_empty());
}

#[test]
fn production_execution_sources_contain_no_synthetic_workspace_fallback() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest.join("src"),
        manifest
            .parent()
            .expect("workspace crates directory should exist")
            .join("desktoplab-tool-gateway/src"),
    ];
    let forbidden = [
        "workspace.desktoplab",
        "/repo/desktoplab",
        "workspace.unavailable",
        "workspace.local",
        "current_exe()",
    ];

    for source in roots.iter().flat_map(|root| rust_sources(root)) {
        let contents = std::fs::read_to_string(&source).expect("production source should read");
        for value in forbidden {
            assert!(
                !contents.contains(value),
                "{} contains forbidden synthetic workspace fallback {value}",
                source.display()
            );
        }
    }
}

#[test]
fn workspace_absence_truth_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_workspace_absence_truth.rs",
        include_str!("local_api_workspace_absence_truth.rs"),
        180,
    )
    .expect("workspace absence truth test should stay focused");
}

fn rust_sources(root: &Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(root).expect("source directory should read") {
        let path = entry.expect("source entry should read").path();
        if path.is_dir() {
            sources.extend(rust_sources(&path));
        } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs") {
            sources.push(path);
        }
    }
    sources
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
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
