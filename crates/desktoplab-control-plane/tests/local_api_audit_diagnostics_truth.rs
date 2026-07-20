use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn local_audit_is_empty_only_before_auditable_actions() {
    let mut router = LocalApiRouter::default();

    let empty = route_json(&mut router, "GET", "/v1/audit/local", "");
    assert_eq!(empty["records"].as_array().unwrap().len(), 0);

    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"providerToken":"sk-secret-value"}"#,
    );

    let audit = route_json(&mut router, "GET", "/v1/audit/local", "");
    let records = audit["records"]
        .as_array()
        .expect("records should be array");
    assert!(
        records
            .iter()
            .any(|record| record["action"] == "runtime_install"),
        "{audit}"
    );
    assert!(
        audit["redactedExport"]
            .as_str()
            .unwrap()
            .contains("[REDACTED]"),
        "{audit}"
    );
    assert!(
        !audit.to_string().contains("sk-secret-value"),
        "audit response leaked secret: {audit}"
    );
}

#[test]
fn diagnostics_reflect_setup_jobs_and_workspace_state() {
    let fixture = git_workspace_fixture();
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(32);

    let fresh = route_json(&mut router, "GET", "/v1/diagnostics", "");
    assert_eq!(fresh["state"], "blocked");
    assert!(
        service_state(&fresh, "runtime") == Some("blocked"),
        "{fresh}"
    );

    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let in_progress = route_json(&mut router, "GET", "/v1/diagnostics", "");
    assert_eq!(in_progress["state"], "degraded");
    assert!(
        service_state(&in_progress, "job") == Some("degraded"),
        "{in_progress}"
    );
    assert_eq!(
        in_progress["bundlePreview"]["setup"]["runtimeId"],
        "runtime.ollama"
    );
    assert_eq!(
        in_progress["bundlePreview"]["setup"]["modelId"],
        "model.gemma4-12b-q4"
    );
    assert_eq!(
        in_progress["bundlePreview"]["setup"]["pipelineState"],
        "runtime_installing"
    );
    assert!(
        in_progress["bundlePreview"]["hardware"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fact| fact["label"] == "OS"),
        "{in_progress}"
    );
    assert!(
        in_progress["bundlePreview"]["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|job| job["kind"] == "model.download" && job["state"] == "blocked"),
        "{in_progress}"
    );
    assert!(
        !in_progress.to_string().contains("/Users/"),
        "diagnostics leaked protected path: {in_progress}"
    );

    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&fixture.path()),
    );
    let ready = route_json(&mut router, "GET", "/v1/diagnostics", "");
    assert_eq!(ready["state"], "ready");
    assert!(
        ready["bundlePreview"]["summary"]
            .as_str()
            .unwrap()
            .contains("setup=ready"),
        "{ready}"
    );
}

fn git_workspace_fixture() -> TempDir {
    let fixture = TempDir::new().expect("workspace fixture should exist");
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(fixture.path())
        .status()
        .expect("git init should run");
    assert!(status.success(), "git init should succeed");
    fixture
}

fn service_state<'a>(snapshot: &'a Value, family: &str) -> Option<&'a str> {
    snapshot["services"].as_array()?.iter().find_map(|service| {
        if service["family"] == family {
            service["state"].as_str()
        } else {
            None
        }
    })
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
