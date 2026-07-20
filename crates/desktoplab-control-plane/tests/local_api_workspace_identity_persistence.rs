use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn duplicate_basename_workspace_identities_survive_restart() {
    let fixture = TempDir::new().unwrap();
    let db_path = fixture.path().join("desktoplab.sqlite");
    let first_root = fixture.path().join("first").join("repo");
    let second_root = fixture.path().join("second").join("repo");
    create_repo(&first_root);
    create_repo(&second_root);
    std::fs::write(first_root.join("first-only.md"), "first\n").unwrap();
    std::fs::write(second_root.join("second-only.md"), "second\n").unwrap();

    let mut router = LocalApiRouter::with_storage_path(&db_path).unwrap();
    mark_setup_ready(&mut router);
    let first = open_workspace(&mut router, &first_root);
    let second = open_workspace(&mut router, &second_root);
    let first_git = route_json(
        &mut router,
        "GET",
        &format!(
            "/v1/git/operations?workspace_id={}",
            first["workspaceId"].as_str().unwrap()
        ),
        "",
    );
    assert_eq!(first_git["workspaceId"], first["workspaceId"]);
    assert!(
        first_git["changedFiles"]
            .to_string()
            .contains("first-only.md")
    );
    assert!(
        !first_git["changedFiles"]
            .to_string()
            .contains("second-only.md")
    );
    drop(router);

    let mut restarted = LocalApiRouter::with_storage_path(&db_path).unwrap();
    let state = route_json(&mut restarted, "GET", "/v1/app/state", "");
    assert_eq!(state["workspaces"].as_array().unwrap().len(), 2);
    assert_eq!(
        open_workspace(&mut restarted, &first_root)["workspaceId"],
        first["workspaceId"]
    );
    assert_eq!(
        open_workspace(&mut restarted, &second_root)["workspaceId"],
        second["workspaceId"]
    );
}

#[test]
fn workspace_identity_resolver_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/workspace_identity.rs",
        include_str!("../src/router/workspace_identity.rs"),
        90,
    )
    .unwrap();
}

fn open_workspace(router: &mut LocalApiRouter, root: &std::path::Path) -> Value {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&root),
    )
}

fn create_repo(root: &std::path::Path) {
    std::fs::create_dir_all(root).unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
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
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
