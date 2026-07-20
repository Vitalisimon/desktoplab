use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn runtime_routes_reflect_service_inventory_and_install_planner() {
    let mut router = LocalApiRouter::default();

    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");
    assert_eq!(inventory["source"], "service_backed");
    assert_eq!(inventory["runtimes"][0]["runtimeId"], "runtime.ollama");
    assert_eq!(inventory["runtimes"][0]["install"]["supported"], true);
    assert_eq!(inventory["runtimes"][0]["ownership"], "user_owned");

    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );
    assert_eq!(install["source"], "service_backed");
    assert_eq!(install["runtimeId"], "runtime.ollama");
    assert_ne!(install["state"], "queued");
    assert!(matches!(
        install["state"].as_str(),
        Some("completed" | "blocked" | "external_guided" | "failed")
    ));
    assert!(install["executionEvidence"].as_str().is_some());
    assert!(install["jobId"].as_str().unwrap().starts_with("job."));
}

#[test]
fn runtime_inventory_marks_ollama_managed_only_with_desktoplab_owner_marker() {
    let fixture = TempDir::new().expect("fixture should exist");
    let marker = fixture.path().join("runtime/ollama-owned-by-desktoplab");
    std::fs::create_dir_all(marker.parent().expect("marker parent should exist"))
        .expect("runtime marker parent should exist");
    std::fs::write(&marker, "desktop-session-1\n").expect("marker should write");
    let mut router =
        LocalApiRouter::default().with_managed_runtime_ownership(&marker, "desktop-session-1");

    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");

    let ollama = inventory["runtimes"]
        .as_array()
        .expect("runtimes")
        .iter()
        .find(|runtime| runtime["runtimeId"] == "runtime.ollama")
        .expect("Ollama runtime");
    assert_eq!(ollama["ownership"], "desktoplab_managed");
}

#[test]
fn runtime_install_returns_blocked_state_from_planner_when_offline() {
    let mut router = LocalApiRouter::default();

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64}"#,
    );

    assert_eq!(blocked["source"], "service_backed");
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["retryClass"], "offline");
    assert_eq!(blocked["blockedReason"], "network unavailable");
}

#[test]
fn runtime_inventory_does_not_claim_unimplemented_update_or_uninstall() {
    let mut router = LocalApiRouter::default();

    let inventory = route_json(&mut router, "GET", "/v1/runtimes", "");
    let runtimes = inventory["runtimes"].as_array().expect("runtimes");
    let ollama = runtimes
        .iter()
        .find(|runtime| runtime["runtimeId"] == "runtime.ollama")
        .expect("Ollama runtime");
    let lm_studio = runtimes
        .iter()
        .find(|runtime| runtime["runtimeId"] == "runtime.lm-studio")
        .expect("LM Studio runtime");

    assert_eq!(ollama["lifecycle"]["update"]["state"], "packaging_managed");
    assert_eq!(
        ollama["lifecycle"]["uninstall"]["state"],
        "packaging_managed"
    );
    assert_eq!(lm_studio["ownership"], "externally_managed");
    assert_eq!(lm_studio["install"]["supported"], false);
    assert_eq!(lm_studio["lifecycle"]["update"]["state"], "blocked");
    assert_eq!(lm_studio["lifecycle"]["uninstall"]["state"], "blocked");
}

#[test]
fn local_api_runtime_install_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_runtime_install.rs",
        include_str!("local_api_runtime_install.rs"),
        180,
    )
    .expect("runtime install route test should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/runtime_routes.rs",
        include_str!("../src/runtime_routes.rs"),
        220,
    )
    .expect("runtime route source should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
