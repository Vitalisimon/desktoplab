use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn local_route_declares_complete_backend_support_contract() {
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    let _workspace = open_test_workspace(&mut router);

    let workspace = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let contract = &workspace["route"]["backendSupportContract"];

    assert_eq!(contract["backendId"], "backend.ollama");
    assert_eq!(contract["supportState"], "contract_ready");
    assert_eq!(contract["modelLoopOwner"], "desktoplab");
    assert_eq!(contract["canonicalThreadOwner"], "desktoplab");
    assert_eq!(contract["toolOwner"], "desktoplab");
    assert_eq!(contract["approvalOwner"], "desktoplab");
    assert_eq!(contract["contextOwner"], "desktoplab");
    assert_eq!(contract["compactionOwner"], "desktoplab");
    assert_eq!(contract["transcriptMirror"], "canonical");
    assert!(
        contract["unsupportedSurfaces"]
            .as_array()
            .unwrap()
            .contains(&Value::String("provider_owned_session".to_string())),
        "{contract}"
    );
}

#[test]
fn external_codex_backend_declares_desktoplab_session_ownership() {
    let mut router = LocalApiRouter::default();

    let external = route_json(&mut router, "GET", "/v1/external-backends", "");
    let contract = &external["backends"][0]["backendSupportContract"];

    assert_eq!(contract["backendId"], "backend.codex");
    assert_eq!(contract["supportState"], "contract_ready");
    assert_eq!(contract["modelLoopOwner"], "external_backend");
    assert_eq!(contract["canonicalThreadOwner"], "desktoplab");
    assert_eq!(contract["approvalOwner"], "desktoplab");
    assert_eq!(contract["contextOwner"], "desktoplab");
    assert_eq!(
        contract["transcriptMirror"],
        "mirrored_from_external_events"
    );
    assert!(
        contract["unsupportedSurfaces"]
            .as_array()
            .unwrap()
            .contains(&Value::String("automatic_repository_egress".to_string())),
        "{contract}"
    );
}

#[test]
fn known_backend_contracts_fail_closed_when_contract_state_is_missing() {
    let mut router = LocalApiRouter::default();
    let external = route_json(&mut router, "GET", "/v1/external-backends", "");
    let backends = external["backends"].as_array().unwrap();
    let codex = backends
        .iter()
        .find(|backend| backend["backendId"] == "backend.codex")
        .expect("codex backend should be listed");

    assert_eq!(
        codex["backendSupportContract"]["supportState"],
        "contract_ready"
    );
    for field in [
        "modelLoopOwner",
        "canonicalThreadOwner",
        "toolOwner",
        "approvalOwner",
        "contextOwner",
        "compactionOwner",
        "transcriptMirror",
        "unsupportedSurfaces",
    ] {
        assert!(
            !codex["backendSupportContract"][field].is_null(),
            "{field} must be present"
        );
    }
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    post(
        router,
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    post(router, "/v1/setup/complete", "{}");
}

fn post(router: &mut LocalApiRouter, path: &str, body: &str) {
    let _ = route_json(router, "POST", path, body);
}

fn open_test_workspace(router: &mut LocalApiRouter) -> TempDir {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let root = fixture.path().join("desktoplab");
    std::fs::create_dir(&root).expect("workspace should be created");
    post(
        router,
        "/v1/workspaces/open",
        &xtask::test_http::workspace_initialize_body(&root),
    );
    fixture
}
