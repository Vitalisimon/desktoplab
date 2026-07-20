use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn critical_routes_must_disclose_fixture_or_service_source() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();

    let responses = [
        ("GET", "/v1/setup/preview", ""),
        (
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
        ),
        ("GET", "/v1/setup/catalog-refresh", ""),
        ("POST", "/v1/setup/catalog-refresh", "{}"),
        ("GET", "/v1/providers", ""),
        (
            "POST",
            "/v1/providers/provider.openai/connect",
            r#"{"accountMode":"api_key_billing"}"#,
        ),
        ("GET", "/v1/runtimes", ""),
        ("POST", "/v1/runtimes/runtime.ollama/install", "{}"),
        ("GET", "/v1/models", ""),
        ("POST", "/v1/models/model.gemma4-12b-q4/download", "{}"),
        ("GET", "/v1/jobs", ""),
        ("GET", "/v1/events/replay", ""),
        ("GET", "/v1/approvals", ""),
    ];

    for (method, path, body) in responses {
        let response = router
            .route(method, path, body)
            .unwrap_or_else(|| panic!("{method} {path} should be routed"));
        let value: Value = serde_json::from_str(response.body())
            .unwrap_or_else(|error| panic!("{method} {path} returned invalid json: {error}"));

        assert!(
            has_service_or_fixture_marker(&value),
            "{method} {path} returned critical local API JSON without service or dry-run boundary: {value}"
        );
    }
}

#[test]
fn control_and_unfinished_repair_routes_do_not_return_canned_acceptance() {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace_root).expect("workspace should be created");
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&workspace_root),
    );
    let session = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        r#"{"workspaceId":"workspace.desktoplab","executionBackendId":"backend.ollama","initialPrompt":"inspect","stream":true}"#,
    );
    let session_id = session["sessionId"].as_str().expect("session id");

    let session_control = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/control"),
        r#"{"action":"pause"}"#,
    );
    assert_eq!(session_control["state"], "paused");
    assert_ne!(session_control["accepted"], true);

    let repair = route_json(
        &mut router,
        "POST",
        "/v1/diagnostics/repairs/repair.disk/run",
        "{}",
    );
    assert_eq!(repair["status"], "blocked");
    assert_ne!(repair["accepted"], true);

    let worktree_cleanup = route_json(
        &mut router,
        "POST",
        "/v1/git/worktrees/worktree.1/cleanup",
        "{}",
    );
    assert_eq!(worktree_cleanup["status"], "blocked");
    assert_ne!(worktree_cleanup["accepted"], true);
}

#[test]
fn canned_route_guard_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_no_canned_critical_routes.rs",
        include_str!("local_api_no_canned_critical_routes.rs"),
        220,
    )
    .expect("canned-route guard should stay focused");
}

fn has_service_or_fixture_marker(value: &Value) -> bool {
    contains_pair(value, "source", "service_backed")
        || contains_pair(value, "source", "dry_run_contract_fixture")
        || contains_pair(value, "implementationState", "final_hardening_required")
}

fn contains_pair(value: &Value, key: &str, expected: &str) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(candidate_key, candidate_value)| {
            (candidate_key == key && candidate_value == expected)
                || contains_pair(candidate_value, key, expected)
        }),
        Value::Array(values) => values
            .iter()
            .any(|candidate| contains_pair(candidate, key, expected)),
        _ => false,
    }
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
